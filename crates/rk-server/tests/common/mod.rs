use rk_server::auth::hash_token;
use sqlx::PgPool;
use uuid::Uuid;

/// Create a test user directly in the database. Returns user_id.
/// Each call generates a unique username to avoid collisions in multi-user tests.
pub async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    let username = format!("testuser_{}", &user_id.to_string()[..8]);
    sqlx::query("INSERT INTO users (id, github_id, username, email) VALUES ($1, $2, $3, $4)")
        .bind(user_id)
        .bind(rand::random::<i64>().abs())
        .bind(&username)
        .bind(Some(format!("{}@example.com", username)))
        .execute(pool)
        .await
        .expect("Failed to create test user");
    user_id
}

/// Create a bearer token for a test user. Returns the raw token string.
pub async fn create_test_token(pool: &PgPool, user_id: Uuid) -> String {
    let raw_token = format!("rk_test_{}", Uuid::new_v4());
    let token_hash = hash_token(&raw_token);

    let device_id = Uuid::new_v4();
    sqlx::query("INSERT INTO devices (id, user_id, name) VALUES ($1, $2, $3)")
        .bind(device_id)
        .bind(user_id)
        .bind("test-device")
        .execute(pool)
        .await
        .expect("Failed to create test device");

    sqlx::query(
        "INSERT INTO auth_tokens (token_hash, user_id, device_id, expires_at) \
         VALUES ($1, $2, $3, NOW() + INTERVAL '90 days')",
    )
    .bind(&token_hash)
    .bind(user_id)
    .bind(device_id)
    .execute(pool)
    .await
    .expect("Failed to create test token");

    raw_token
}

/// Create a default "Personal" board for a user. Returns board UUID.
pub async fn create_test_board(pool: &PgPool, user_id: Uuid) -> Uuid {
    let board_uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO boards (uuid, user_id, name, position) VALUES ($1, $2, 'Personal', 0)",
    )
    .bind(board_uuid)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("Failed to create test board");
    board_uuid
}
