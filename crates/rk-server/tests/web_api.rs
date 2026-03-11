mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Extension;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;

// We'll add tests here as we build endpoints.
// For now, verify the test infrastructure works.

#[sqlx::test(migrations = "./migrations")]
async fn test_setup_works(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let _board_uuid = common::create_test_board(&pool, user_id).await;
    assert!(token.starts_with("rk_test_"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_me(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;

    let app = axum::Router::new()
        .route(
            "/api/v1/me",
            axum::routing::get(rk_server::routes::web::get_me),
        )
        .layer(Extension(pool));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["username"], "testuser");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_get_me_unauthorized(pool: PgPool) {
    let app = axum::Router::new()
        .route(
            "/api/v1/me",
            axum::routing::get(rk_server::routes::web::get_me),
        )
        .layer(Extension(pool));

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_list_boards(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let _board = common::create_test_board(&pool, user_id).await;

    let app = build_app(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/boards")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = parse_body(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["name"], "Personal");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_create_task(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let board_uuid = common::create_test_board(&pool, user_id).await;

    let app = build_app(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "board_uuid": board_uuid.to_string(),
                        "title": "Test task",
                        "priority": "High"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = parse_body(resp).await;
    assert_eq!(body["title"], "Test task");
    assert_eq!(body["priority"], "High");
    assert_eq!(body["column"], "todo");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_task_ownership_enforced(pool: PgPool) {
    // Create two users
    let user1 = common::create_test_user(&pool).await;
    let token1 = common::create_test_token(&pool, user1).await;
    let board1 = common::create_test_board(&pool, user1).await;

    let user2 = common::create_test_user(&pool).await;
    let token2 = common::create_test_token(&pool, user2).await;

    // User 1 creates a task
    let app = build_app(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("Authorization", format!("Bearer {token1}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "board_uuid": board1.to_string(),
                        "title": "User1's task"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let task: serde_json::Value = parse_body(resp).await;
    let task_uuid = task["uuid"].as_str().unwrap();

    // User 2 tries to delete it -- should fail
    let app = build_app(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/tasks/{task_uuid}"))
                .header("Authorization", format!("Bearer {token2}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Suppress unused variable warning
    let _ = token2;
}

// Helper functions for tests
fn build_app(pool: PgPool) -> axum::Router {
    use axum::routing::{get, patch, post};
    axum::Router::new()
        .route("/api/v1/me", get(rk_server::routes::web::get_me))
        .route("/api/v1/boards", get(rk_server::routes::web::list_boards))
        .route(
            "/api/v1/boards/{uuid}",
            get(rk_server::routes::web::get_board),
        )
        .route("/api/v1/tasks", post(rk_server::routes::web::create_task))
        .route(
            "/api/v1/tasks/{uuid}",
            patch(rk_server::routes::web::update_task).delete(rk_server::routes::web::delete_task),
        )
        .route("/api/v1/tags", post(rk_server::routes::web::create_tag))
        .route(
            "/api/v1/tags/{uuid}",
            patch(rk_server::routes::web::update_tag).delete(rk_server::routes::web::delete_tag),
        )
        .layer(Extension(pool))
}

async fn parse_body(resp: axum::http::Response<axum::body::Body>) -> serde_json::Value {
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}
