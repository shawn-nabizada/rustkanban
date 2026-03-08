use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use oauth2::{AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl};
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::pages::{ErrorTemplate, HtmlTemplate, LoginTokenTemplate};
use crate::auth::{generate_token, hash_token};
use crate::config::Config;
use crate::session::SESSION_KEY_USER_ID;

/// Shared application state passed to all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    pub http_client: reqwest::Client,
}

/// Temporary record stored between login redirect and OAuth callback.
pub struct PendingLogin {
    pub redirect_port: Option<u16>,
    pub device_name: String,
    #[allow(dead_code)]
    pub headless: bool,
    pub created_at: tokio::time::Instant,
}

/// Query parameters for the `/login` endpoint.
#[derive(Deserialize)]
pub struct LoginQuery {
    pub redirect_port: Option<u16>,
    pub device_name: Option<String>,
    pub mode: Option<String>,
}

/// Query parameters received from GitHub on the `/auth/callback` endpoint.
#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

/// GitHub user profile returned by the GitHub API.
#[derive(Deserialize)]
struct GitHubUser {
    id: i64,
    login: String,
    email: Option<String>,
}

/// `GET /login` -- Initiates GitHub OAuth flow.
///
/// Query parameters:
/// - `redirect_port` (optional): local port for CLI callback redirect
/// - `device_name` (optional): human-readable device name (defaults to "unknown")
/// - `mode` (optional): set to "headless" to display the token in the browser
pub async fn login(State(state): State<AppState>, Query(params): Query<LoginQuery>) -> Response {
    let client =
        oauth2::basic::BasicClient::new(ClientId::new(state.config.github_client_id.clone()))
            .set_client_secret(ClientSecret::new(state.config.github_client_secret.clone()))
            .set_auth_uri(
                AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
                    .expect("Invalid auth URL"),
            )
            .set_token_uri(
                TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
                    .expect("Invalid token URL"),
            )
            .set_redirect_uri(
                RedirectUrl::new(format!("{}/auth/callback", state.config.server_url))
                    .expect("Invalid redirect URL"),
            );

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    let pending = PendingLogin {
        redirect_port: params.redirect_port,
        device_name: params.device_name.unwrap_or_else(|| "unknown".to_string()),
        headless: params.mode.as_deref() == Some("headless"),
        created_at: tokio::time::Instant::now(),
    };

    {
        let mut logins = state.pending_logins.lock().await;
        // Evict entries older than 10 minutes to prevent unbounded growth
        logins.retain(|_, v| v.created_at.elapsed() < std::time::Duration::from_secs(600));
        logins.insert(csrf_token.secret().clone(), pending);
    }

    Redirect::temporary(auth_url.as_str()).into_response()
}

/// `GET /auth/callback` -- GitHub OAuth callback handler.
///
/// Exchanges the authorization code for a GitHub access token, fetches the
/// user profile, upserts the user in the database, creates a device and
/// bearer token, then either redirects to the CLI or displays the token.
pub async fn callback(
    State(state): State<AppState>,
    session: tower_sessions::Session,
    Query(params): Query<CallbackQuery>,
) -> Response {
    // Look up and remove the pending login by CSRF state
    let pending = {
        let mut logins = state.pending_logins.lock().await;
        match logins.remove(&params.state) {
            Some(p) => p,
            None => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    HtmlTemplate(ErrorTemplate {
                        title: "Authentication Failed".into(),
                        message: "Invalid or expired login session. Please try again.".into(),
                        logged_in: false,
                    }),
                )
                    .into_response();
            }
        }
    };

    // Exchange authorization code for GitHub access token
    let github_token =
        match exchange_code_for_token(&state.http_client, &state.config, &params.code).await {
            Ok(token) => token,
            Err(e) => {
                tracing::error!("GitHub token exchange failed: {e}");
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    HtmlTemplate(ErrorTemplate {
                        title: "Authentication Failed".into(),
                        message: "Failed to communicate with GitHub. Please try again.".into(),
                        logged_in: false,
                    }),
                )
                    .into_response();
            }
        };

    // Fetch GitHub user profile
    let gh_user = match fetch_github_user(&state.http_client, &github_token).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("GitHub user fetch failed: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                HtmlTemplate(ErrorTemplate {
                    title: "Authentication Failed".into(),
                    message: "Failed to retrieve your GitHub profile. Please try again.".into(),
                    logged_in: false,
                }),
            )
                .into_response();
        }
    };

    // Upsert user in database
    let user_id = match upsert_user(&state.pool, &gh_user).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("User upsert failed: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                HtmlTemplate(ErrorTemplate {
                    title: "Authentication Failed".into(),
                    message: "Failed to set up your account. Please try again.".into(),
                    logged_in: false,
                }),
            )
                .into_response();
        }
    };

    // Set session for browser pages
    if let Err(e) = session
        .insert(SESSION_KEY_USER_ID, user_id.to_string())
        .await
    {
        tracing::error!("Session insert failed: {e}");
    }

    // Browser login: only needs session cookie, no device/token
    let is_browser_login = pending.redirect_port.is_none() && !pending.headless;
    if is_browser_login {
        return Redirect::temporary("/account").into_response();
    }

    // CLI login: create device + bearer token
    let device_id = Uuid::new_v4();
    if let Err(e) = sqlx::query("INSERT INTO devices (id, user_id, name) VALUES ($1, $2, $3)")
        .bind(device_id)
        .bind(user_id)
        .bind(&pending.device_name)
        .execute(&state.pool)
        .await
    {
        tracing::error!("Device creation failed: {e}");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            HtmlTemplate(ErrorTemplate {
                title: "Authentication Failed".into(),
                message: "Failed to set up your device. Please try again.".into(),
                logged_in: false,
            }),
        )
            .into_response();
    }

    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);

    if let Err(e) = sqlx::query(
        "INSERT INTO auth_tokens (token_hash, user_id, device_id, expires_at) \
         VALUES ($1, $2, $3, NOW() + INTERVAL '90 days')",
    )
    .bind(&token_hash)
    .bind(user_id)
    .bind(device_id)
    .execute(&state.pool)
    .await
    {
        tracing::error!("Token creation failed: {e}");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            HtmlTemplate(ErrorTemplate {
                title: "Authentication Failed".into(),
                message: "Failed to create authentication token. Please try again.".into(),
                logged_in: false,
            }),
        )
            .into_response();
    }

    if let Some(port) = pending.redirect_port {
        let redirect_url = format!(
            "http://localhost:{port}/callback?token={}&device_id={device_id}",
            raw_token
        );
        Redirect::temporary(&redirect_url).into_response()
    } else {
        // Headless CLI mode: render token in the browser for manual copy
        HtmlTemplate(LoginTokenTemplate {
            token: raw_token,
            device_id: device_id.to_string(),
            logged_in: true,
        })
        .into_response()
    }
}

/// Exchange a GitHub authorization code for an access token using the
/// GitHub OAuth token endpoint directly via reqwest.
async fn exchange_code_for_token(
    client: &reqwest::Client,
    config: &Config,
    code: &str,
) -> Result<String, String> {
    let resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": config.github_client_id,
            "client_secret": config.github_client_secret,
            "code": code,
        }))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub returned status {}", resp.status()));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
    }

    let body: TokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    if let Some(err) = body.error {
        let desc = body.error_description.unwrap_or_default();
        return Err(format!("GitHub OAuth error: {err} - {desc}"));
    }

    body.access_token
        .ok_or_else(|| "No access_token in GitHub response".to_string())
}

/// Fetch the authenticated user's GitHub profile.
async fn fetch_github_user(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<GitHubUser, String> {
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("User-Agent", "RustKanban-Server")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned status {}", resp.status()));
    }

    resp.json::<GitHubUser>()
        .await
        .map_err(|e| format!("Failed to parse GitHub user: {e}"))
}

/// Insert a new user or update an existing one (matched by github_id).
/// Returns the user's UUID.
async fn upsert_user(pool: &PgPool, gh_user: &GitHubUser) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid,)>(
        "INSERT INTO users (id, github_id, username, email) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (github_id) DO UPDATE SET username = EXCLUDED.username, email = EXCLUDED.email \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(gh_user.id)
    .bind(&gh_user.login)
    .bind(&gh_user.email)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}
