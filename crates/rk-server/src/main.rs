mod auth;
mod config;
mod error;
mod purge;
mod routes;

use axum::{routing::get, routing::post, Extension, Router};
use sqlx::migrate::Migrator;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;
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

    let state = routes::auth::AppState {
        pool: pool.clone(),
        config: config.clone(),
        pending_logins: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/", get(routes::pages::home))
        .route("/health", get(|| async { "OK" }))
        .route("/login", get(routes::auth::login))
        .route("/auth/callback", get(routes::auth::callback))
        .route("/api/v1/sync/pull", post(routes::sync::pull))
        .route("/api/v1/sync/push", post(routes::sync::push))
        .route("/api/v1/sync", post(routes::sync::combined))
        .route("/account/devices", get(routes::account::list_devices))
        .route(
            "/account/devices/:id/revoke",
            post(routes::account::revoke_device),
        )
        .route("/account/delete", post(routes::account::delete_account))
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10 MB
        .layer(Extension(pool.clone()));

    purge::spawn_purge_job(pool);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
