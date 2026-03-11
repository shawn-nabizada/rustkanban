use rk_server::{config, purge, routes};

use axum::{response::IntoResponse, routing::get, routing::post, Extension, Router};
use sha2::{Digest, Sha512};
use sqlx::migrate::Migrator;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::EnvFilter;

use config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
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

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/login", get(routes::auth::login))
        .route("/login/success", get(routes::pages::login_success))
        .route("/account", get(routes::pages::account))
        .route("/auth/callback", get(routes::auth::callback))
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
        // Sharing API
        .route(
            "/api/v1/boards/{uuid}/shares",
            post(routes::shares::create_share).get(routes::shares::list_shares),
        )
        .route(
            "/api/v1/shares/{id}",
            axum::routing::delete(routes::shares::delete_share),
        )
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
        // Backward compat redirect
        .route(
            "/app",
            get(|| async { axum::response::Redirect::permanent("/").into_response() }),
        )
        // SPA: serve frontend at root, with index.html fallback for client-side routing
        .fallback_service(
            ServeDir::new("crates/rk-server/frontend/dist")
                .not_found_service(ServeFile::new("crates/rk-server/frontend/dist/index.html")),
        )
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10 MB
        .layer(Extension(pool.clone()))
        .layer(session_layer);

    purge::spawn_purge_job(pool);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
