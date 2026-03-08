mod auth;
mod config;
mod error;
mod purge;
mod routes;
mod session;

use axum::{routing::get, routing::post, Extension, Router};
use sha2::{Digest, Sha512};
use sqlx::migrate::Migrator;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::ServeDir;
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

    let migrator = Migrator::new(Path::new("./migrations"))
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
        .route("/", get(routes::pages::home))
        .route("/health", get(|| async { "OK" }))
        .route("/login", get(routes::auth::login))
        .route("/login/success", get(routes::pages::login_success))
        .route("/account", get(routes::pages::account))
        .route("/auth/callback", get(routes::auth::callback))
        .route("/auth/logout", get(routes::pages::logout))
        .route("/api/v1/sync/pull", post(routes::sync::pull))
        .route("/api/v1/sync/push", post(routes::sync::push))
        .route("/api/v1/sync", post(routes::sync::combined))
        .route("/account/devices", get(routes::account::list_devices))
        .route(
            "/account/devices/{id}/revoke",
            post(routes::account::revoke_device),
        )
        .route(
            "/account/devices/{id}/rename",
            post(routes::account::rename_device),
        )
        .route(
            "/account/tokens/create",
            post(routes::account::create_api_token),
        )
        .route(
            "/account/tokens/{id}/revoke",
            post(routes::account::revoke_api_token),
        )
        .route("/account/export", get(routes::account::export_data))
        .route("/account/delete", post(routes::account::delete_account))
        .fallback(routes::pages::not_found)
        .nest_service("/static", ServeDir::new("static"))
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
