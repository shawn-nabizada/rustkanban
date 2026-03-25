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
    assert!(json["username"].as_str().unwrap().starts_with("testuser_"));
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
}

// ───────────────────────────── Task Update ──────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_update_task(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let board_uuid = common::create_test_board(&pool, user_id).await;

    // Create a task
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
                        "title": "Original title"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let task: serde_json::Value = parse_body(resp).await;
    let task_uuid = task["uuid"].as_str().unwrap();

    // Update it
    let app = build_app(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(&format!("/api/v1/tasks/{task_uuid}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Updated title",
                        "priority": "Low",
                        "column": "in_progress"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = parse_body(resp).await;
    assert_eq!(body["title"], "Updated title");
    assert_eq!(body["priority"], "Low");
    assert_eq!(body["column"], "in_progress");
}

// ───────────────────────────── Tag CRUD ─────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_create_and_delete_tag(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;

    // Create tag
    let app = build_app(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tags")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::json!({"name": "bug"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let tag: serde_json::Value = parse_body(resp).await;
    assert_eq!(tag["name"], "bug");
    let tag_uuid = tag["uuid"].as_str().unwrap();

    // Delete tag
    let app = build_app(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/tags/{tag_uuid}"))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ───────────────────────────── Sharing ──────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_create_share_and_access(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let board_uuid = common::create_test_board(&pool, user_id).await;

    let app = build_app_with_shares(pool.clone());

    // Create a share link
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/boards/{board_uuid}/shares"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"permission": "view"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let share: serde_json::Value = parse_body(resp).await;
    assert_eq!(share["permission"], "view");
    let share_token = share["token"].as_str().unwrap();

    // Access the shared board (no auth required)
    let app = build_app_with_shares(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/shared/{share_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let shared: serde_json::Value = parse_body(resp).await;
    assert_eq!(shared["board"]["name"], "Personal");
    assert_eq!(shared["permission"], "view");
    assert!(shared["owner_username"]
        .as_str()
        .unwrap()
        .starts_with("testuser_"));
}

#[sqlx::test(migrations = "./migrations")]
async fn test_shared_board_edit_permissions(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let board_uuid = common::create_test_board(&pool, user_id).await;

    // Create an edit share
    let app = build_app_with_shares(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/boards/{board_uuid}/shares"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"permission": "edit"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let share: serde_json::Value = parse_body(resp).await;
    let share_token = share["token"].as_str().unwrap();

    // Create a task via the share link (requires auth + edit permission)
    let app = build_app_with_shares(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/shared/{share_token}/tasks"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"title": "Shared task"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let task: serde_json::Value = parse_body(resp).await;
    assert_eq!(task["title"], "Shared task");
    assert_eq!(task["column"], "todo");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_shared_board_invalid_token(pool: PgPool) {
    let app = build_app_with_shares(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/shared/nonexistent_token_123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ───────────────────────────── Sync ─────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_sync_pull_empty(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let _board = common::create_test_board(&pool, user_id).await;

    let app = build_app_with_sync(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sync/pull")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "tasks": [],
                        "tags": [],
                        "boards": [],
                        "last_synced_at": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = parse_body(resp).await;
    // Should have the board we created
    assert!(!body["boards"].as_array().unwrap().is_empty());
    assert!(body["synced_at"].as_str().is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn test_sync_push_task(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let board_uuid = common::create_test_board(&pool, user_id).await;

    let task_uuid = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let app = build_app_with_sync(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sync/push")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "tasks": [{
                            "uuid": task_uuid,
                            "title": "Synced task",
                            "description": "",
                            "priority": "Medium",
                            "column": "todo",
                            "tags": [],
                            "created_at": now,
                            "updated_at": now,
                            "deleted": false,
                            "board_uuid": board_uuid.to_string()
                        }],
                        "tags": [],
                        "boards": [],
                        "last_synced_at": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = parse_body(resp).await;
    assert!(body["synced_at"].as_str().is_some());

    // Verify the task was stored by pulling
    let app = build_app_with_sync(pool.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/sync/pull")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "tasks": [],
                        "tags": [],
                        "boards": [],
                        "last_synced_at": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let body: serde_json::Value = parse_body(resp).await;
    let tasks = body["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"], "Synced task");
}

// ───────────────────────────── CSRF Integration ─────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn test_csrf_blocks_post_without_header(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let _token = common::create_test_token(&pool, user_id).await;
    let board_uuid = common::create_test_board(&pool, user_id).await;

    // Build app WITH CSRF middleware (simulates production behavior)
    let app = build_app_with_csrf(pool.clone());

    // POST without Bearer token and without X-Requested-With header → 403
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "board_uuid": board_uuid.to_string(),
                        "title": "Should be blocked"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = parse_body(resp).await;
    assert_eq!(body["error"], "CSRF validation failed");
}

#[sqlx::test(migrations = "./migrations")]
async fn test_csrf_allows_bearer_token(pool: PgPool) {
    let user_id = common::create_test_user(&pool).await;
    let token = common::create_test_token(&pool, user_id).await;
    let board_uuid = common::create_test_board(&pool, user_id).await;

    // Build app WITH CSRF middleware
    let app = build_app_with_csrf(pool.clone());

    // POST with Bearer token (no X-Requested-With) → should pass CSRF check
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
                        "title": "Allowed via Bearer"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn test_csrf_allows_xhr_header_without_bearer(pool: PgPool) {
    let _user_id = common::create_test_user(&pool).await;

    let app = build_app_with_csrf(pool.clone());

    // POST with X-Requested-With but NO Bearer token — CSRF middleware should
    // allow it through (the request will then fail at auth, not at CSRF).
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks")
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Should pass CSRF but fail auth"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should NOT be 403 (CSRF) — it should be 401 (unauthorized) because
    // the X-Requested-With header satisfies CSRF but there's no auth token.
    assert_ne!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ───────────────────────────── Helpers ──────────────────────────────────────

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

fn build_app_with_shares(pool: PgPool) -> axum::Router {
    use axum::routing::{get, patch, post};
    build_app(pool.clone())
        .route(
            "/api/v1/boards/{uuid}/shares",
            post(rk_server::routes::shares::create_share)
                .get(rk_server::routes::shares::list_shares),
        )
        .route(
            "/api/v1/shares/{id}",
            axum::routing::delete(rk_server::routes::shares::delete_share),
        )
        .route(
            "/api/v1/shared/{token}",
            get(rk_server::routes::shares::get_shared_board),
        )
        .route(
            "/api/v1/shared/{token}/tasks",
            post(rk_server::routes::shares::create_shared_task),
        )
        .route(
            "/api/v1/shared/{token}/tasks/{uuid}",
            patch(rk_server::routes::shares::update_shared_task)
                .delete(rk_server::routes::shares::delete_shared_task),
        )
        .layer(Extension(pool))
}

fn build_app_with_sync(pool: PgPool) -> axum::Router {
    use axum::routing::post;
    build_app(pool.clone())
        .route("/api/v1/sync/pull", post(rk_server::routes::sync::pull))
        .route("/api/v1/sync/push", post(rk_server::routes::sync::push))
        .route("/api/v1/sync", post(rk_server::routes::sync::combined))
        .layer(Extension(pool))
}

fn build_app_with_csrf(pool: PgPool) -> axum::Router {
    use axum::middleware;
    build_app(pool).layer(middleware::from_fn(rk_server::csrf::csrf_protection))
}

async fn parse_body(resp: axum::http::Response<axum::body::Body>) -> serde_json::Value {
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}
