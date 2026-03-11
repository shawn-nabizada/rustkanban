use axum::response::{IntoResponse, Redirect, Response};
use tower_sessions::Session;

/// `GET /login/success` — Redirect to SPA after OAuth.
pub async fn login_success() -> Response {
    Redirect::temporary("/").into_response()
}

/// `GET /account` — Backward-compat redirect to SPA account page.
pub async fn account() -> Response {
    Redirect::temporary("/#/account").into_response()
}

/// `GET /auth/logout` — Destroy session and redirect to SPA.
pub async fn logout(session: Session) -> Response {
    let _ = session.delete().await;
    Redirect::temporary("/").into_response()
}
