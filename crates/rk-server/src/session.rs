use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

/// Session key for the authenticated user's ID.
pub const SESSION_KEY_USER_ID: &str = "user_id";

/// Session key for the CSRF protection token.
pub const SESSION_KEY_CSRF_TOKEN: &str = "csrf_token";

/// Extract authenticated user_id from session, or None if not logged in.
pub async fn session_user_id(session: &Session) -> Option<Uuid> {
    let id_str: String = session.get(SESSION_KEY_USER_ID).await.ok()??;
    Uuid::parse_str(&id_str).ok()
}

/// Get or generate a CSRF token for this session.
pub async fn csrf_token(session: &Session) -> String {
    if let Ok(Some(token)) = session.get::<String>(SESSION_KEY_CSRF_TOKEN).await {
        return token;
    }
    let token = Uuid::new_v4().to_string();
    let _ = session.insert(SESSION_KEY_CSRF_TOKEN, &token).await;
    token
}

/// Validate that the CSRF token from a form submission matches the session token.
pub async fn validate_csrf(session: &Session, form_token: &str) -> bool {
    let session_token: Option<String> = session.get(SESSION_KEY_CSRF_TOKEN).await.ok().flatten();
    matches!(session_token, Some(t) if t == form_token)
}

/// Form data containing only a CSRF token (used by account actions).
#[derive(Deserialize)]
pub struct CsrfForm {
    pub csrf_token: String,
}

/// Session key for flash messages.
pub const SESSION_KEY_FLASH: &str = "flash";

/// Flash message with a type (success, error) and text.
#[derive(Serialize, Deserialize, Clone)]
pub struct FlashMessage {
    pub kind: String,
    pub text: String,
}

/// Set a flash message in the session (shown on next page load).
pub async fn set_flash(session: &Session, kind: &str, text: &str) {
    let msg = FlashMessage {
        kind: kind.to_string(),
        text: text.to_string(),
    };
    let _ = session.insert(SESSION_KEY_FLASH, &msg).await;
}

/// Take the flash message from the session (reads and removes it).
pub async fn take_flash(session: &Session) -> Option<FlashMessage> {
    let msg: Option<FlashMessage> = session.get(SESSION_KEY_FLASH).await.ok()?;
    if msg.is_some() {
        let _ = session.remove::<FlashMessage>(SESSION_KEY_FLASH).await;
    }
    msg
}
