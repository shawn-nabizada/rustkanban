use rk_server::{config, csrf, purge, rate_limit, routes};

use axum::{
    middleware, response::IntoResponse, routing::get, routing::post, Extension, Json, Router,
};
use sha2::{Digest, Sha512};
use sqlx::migrate::Migrator;
use sqlx::PgPool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::EnvFilter;

use config::Config;
use rate_limit::{RateLimitConfig, RateLimitLayer};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = Config::from_env();

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Try ./migrations first (Docker), then crate-relative path (cargo run from workspace root)
    let migrations_path = if Path::new("./migrations").is_dir() {
        Path::new("./migrations")
    } else {
        Path::new("crates/rk-server/migrations")
    };
    let migrator = Migrator::new(migrations_path)
        .await
        .expect("Failed to load migrations");

    migrator.run(&pool).await.expect("Failed to run migrations");

    let session_store = PostgresStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("Failed to migrate session store");

    // Derive signing key via SHA-512 (produces exactly 64 bytes regardless of secret length)
    let mut hasher = Sha512::new();
    hasher.update(config.session_secret.as_bytes());
    let key_bytes: [u8; 64] = hasher.finalize().into();
    let signing_key = tower_sessions::cookie::Key::from(&key_bytes);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_signed(signing_key)
        .with_secure(config.server_url.starts_with("https"))
        .with_http_only(true)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(tower_sessions::Expiry::OnInactivity(
            tower_sessions::cookie::time::Duration::days(30),
        ));

    let state = routes::auth::AppState {
        pool: pool.clone(),
        config: config.clone(),
        pending_logins: Arc::new(Mutex::new(HashMap::new())),
        http_client: reqwest::Client::new(),
    };

    let strict_limiter = RateLimitLayer::new(RateLimitConfig::strict());
    let standard_limiter = RateLimitLayer::new(RateLimitConfig::standard());

    // Routes with strict rate limiting (auth endpoints + unauthenticated shared board endpoints)
    let strict_routes = Router::new()
        .route("/login", get(routes::auth::login))
        .route("/auth/callback", get(routes::auth::callback))
        .route(
            "/api/v1/shared/{token}",
            get(routes::shares::get_shared_board),
        )
        .route(
            "/api/v1/shared/{token}/tasks",
            post(routes::shares::create_shared_task),
        )
        .route(
            "/api/v1/shared/{token}/tasks/{uuid}",
            axum::routing::patch(routes::shares::update_shared_task)
                .delete(routes::shares::delete_shared_task),
        )
        .with_state(state.clone())
        .layer(strict_limiter);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/login/success", get(routes::pages::login_success))
        .route("/account", get(routes::pages::account))
        .route("/auth/logout", get(routes::pages::logout))
        .route("/api/v1/sync/pull", post(routes::sync::pull))
        .route("/api/v1/sync/push", post(routes::sync::push))
        .route("/api/v1/sync", post(routes::sync::combined))
        // Web CRUD API
        .route("/api/v1/me", get(routes::web::get_me))
        .route("/api/v1/boards", get(routes::web::list_boards))
        .route("/api/v1/boards/{uuid}", get(routes::web::get_board))
        .route("/api/v1/tasks", post(routes::web::create_task))
        .route(
            "/api/v1/tasks/{uuid}",
            axum::routing::patch(routes::web::update_task).delete(routes::web::delete_task),
        )
        .route("/api/v1/tags", post(routes::web::create_tag))
        .route(
            "/api/v1/tags/{uuid}",
            axum::routing::patch(routes::web::update_tag).delete(routes::web::delete_tag),
        )
        // Account management API (JSON, session-authenticated)
        .route(
            "/api/v1/account/devices",
            get(routes::web_account::list_devices),
        )
        .route(
            "/api/v1/account/devices/{id}",
            axum::routing::patch(routes::web_account::rename_device)
                .delete(routes::web_account::revoke_device),
        )
        .route(
            "/api/v1/account/tokens",
            get(routes::web_account::list_tokens).post(routes::web_account::create_token),
        )
        .route(
            "/api/v1/account/tokens/{id}",
            axum::routing::delete(routes::web_account::revoke_token),
        )
        .route(
            "/api/v1/account/export",
            get(routes::web_account::export_data),
        )
        .route(
            "/api/v1/account",
            axum::routing::delete(routes::web_account::delete_account),
        )
        .route("/api/v1/auth/logout", post(routes::web_account::logout))
        // Sharing API (authenticated share management)
        .route(
            "/api/v1/boards/{uuid}/shares",
            post(routes::shares::create_share).get(routes::shares::list_shares),
        )
        .route(
            "/api/v1/shares/{id}",
            axum::routing::delete(routes::shares::delete_share),
        )
        // Backward compat redirect
        .route(
            "/app",
            get(|| async { axum::response::Redirect::permanent("/").into_response() }),
        )
        // SPA: serve frontend at root, with index.html fallback for client-side routing.
        // Try ./frontend/dist first (Docker), then crate-relative path (cargo run from workspace root).
        .fallback_service({
            let dist_dir = if Path::new("./frontend/dist").is_dir() {
                "./frontend/dist"
            } else {
                "crates/rk-server/frontend/dist"
            };
            ServeDir::new(dist_dir)
                .not_found_service(ServeFile::new(format!("{}/index.html", dist_dir)))
        })
        .with_state(state)
        .merge(strict_routes)
        .layer(middleware::from_fn(csrf::csrf_protection))
        .layer(standard_limiter)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10 MB
        .layer(Extension(pool.clone()))
        .layer(session_layer);

    purge::spawn_purge_job(pool);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!(addr = %addr, "Starting server");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

    tracing::info!("Server shut down gracefully");
}

/// Health check that verifies database connectivity.
async fn health_check(Extension(pool): Extension<PgPool>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => Json(serde_json::json!({"status": "ok", "database": "connected"})).into_response(),
        Err(_) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "degraded", "database": "disconnected"})),
        )
            .into_response(),
    }
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received SIGINT, shutting down"),
        _ = terminate => tracing::info!("Received SIGTERM, shutting down"),
    }
}
