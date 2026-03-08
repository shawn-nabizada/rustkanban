use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Extension;
use sqlx::PgPool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::session::{csrf_token, session_user_id, take_flash};

/// Wrapper that renders an Askama template into an HTML response.
pub(crate) struct HtmlTemplate<T: Template>(pub(crate) T);

impl<T: Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub title: String,
    pub message: String,
    pub logged_in: bool,
}

/// Fallback handler for 404.
pub async fn not_found(session: Session) -> impl IntoResponse {
    let logged_in = session_user_id(&session).await.is_some();
    (
        StatusCode::NOT_FOUND,
        HtmlTemplate(ErrorTemplate {
            title: "Page Not Found".into(),
            message: "The page you're looking for doesn't exist.".into(),
            logged_in,
        }),
    )
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub logged_in: bool,
}

/// `GET /` -- Landing page.
pub async fn home(session: Session) -> impl IntoResponse {
    let logged_in = session_user_id(&session).await.is_some();
    HtmlTemplate(HomeTemplate { logged_in })
}

#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountTemplate {
    pub username: String,
    pub devices: Vec<DeviceView>,
    pub api_tokens: Vec<ApiTokenView>,
    pub new_api_token: Option<String>,
    pub csrf_token: String,
    pub flash_kind: Option<String>,
    pub flash_text: Option<String>,
}

/// View model for a device row on the account page.
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub last_synced_at: Option<String>,
    pub stale: bool,
}

/// View model for an API token row on the account page.
pub struct ApiTokenView {
    pub id: String,
    pub label: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Template)]
#[template(path = "login_success.html")]
pub struct LoginSuccessTemplate {
    pub logged_in: bool,
}

#[derive(Template)]
#[template(path = "login_token.html")]
pub struct LoginTokenTemplate {
    pub token: String,
    pub device_id: String,
    pub logged_in: bool,
}

/// `GET /login/success` -- Shown after successful OAuth login.
pub async fn login_success(session: Session) -> impl IntoResponse {
    let logged_in = session_user_id(&session).await.is_some();
    HtmlTemplate(LoginSuccessTemplate { logged_in })
}

/// `GET /account` -- Authenticated account management page.
pub async fn account(session: Session, Extension(pool): Extension<PgPool>) -> Response {
    let user_id = match session_user_id(&session).await {
        Some(id) => id,
        None => return Redirect::temporary("/login").into_response(),
    };

    let csrf = csrf_token(&session).await;

    // Run all three independent queries concurrently
    let (username_res, devices_res, tokens_res) = tokio::join!(
        sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&pool),
        sqlx::query_as::<_, (Uuid, String, Option<String>, bool)>(
            "SELECT id, name, \
             to_char(last_synced_at, 'YYYY-MM-DD\"T\"HH24:MI:SS'), \
             stale \
             FROM devices WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&pool),
        sqlx::query_as::<_, (Uuid, String, Option<String>, Option<String>, String)>(
            "SELECT id, label, \
             to_char(last_used_at, 'YYYY-MM-DD\"T\"HH24:MI:SS'), \
             to_char(expires_at, 'YYYY-MM-DD\"T\"HH24:MI:SS'), \
             to_char(created_at, 'YYYY-MM-DD\"T\"HH24:MI:SS') \
             FROM api_tokens WHERE user_id = $1 ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&pool),
    );

    let username: String = match username_res {
        Ok(Some(name)) => name,
        _ => return Redirect::temporary("/login").into_response(),
    };

    let devices: Vec<DeviceView> = match devices_res {
        Ok(rows) => rows
            .into_iter()
            .map(|r| DeviceView {
                id: r.0.to_string(),
                name: r.1,
                last_synced_at: r.2,
                stale: r.3,
            })
            .collect(),
        Err(e) => {
            tracing::error!("Failed to fetch devices: {e}");
            return HtmlTemplate(ErrorTemplate {
                title: "Error".into(),
                message: "Failed to load your devices. Please try again.".into(),
                logged_in: true,
            })
            .into_response();
        }
    };

    let token_rows = match tokens_res {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to fetch API tokens: {e}");
            Vec::new()
        }
    };

    let api_tokens: Vec<ApiTokenView> = token_rows
        .into_iter()
        .map(|r| ApiTokenView {
            id: r.0.to_string(),
            label: r.1,
            last_used_at: r.2,
            expires_at: r.3,
            created_at: r.4,
        })
        .collect();

    let new_api_token: Option<String> = session.get("new_api_token").await.ok().flatten();
    if new_api_token.is_some() {
        let _ = session.remove::<String>("new_api_token").await;
    }

    let flash = take_flash(&session).await;

    HtmlTemplate(AccountTemplate {
        username,
        devices,
        api_tokens,
        new_api_token,
        csrf_token: csrf,
        flash_kind: flash.as_ref().map(|f| f.kind.clone()),
        flash_text: flash.as_ref().map(|f| f.text.clone()),
    })
    .into_response()
}

/// `GET /auth/logout` -- Destroy session and redirect to home.
pub async fn logout(session: Session) -> Response {
    let _ = session.delete().await;
    Redirect::temporary("/").into_response()
}
