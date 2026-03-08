use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// Wrapper that renders an Askama template into an HTML response.
struct HtmlTemplate<T: Template>(T);

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
#[template(path = "home.html")]
pub struct HomeTemplate;

/// `GET /` -- Landing page.
pub async fn home() -> impl IntoResponse {
    HtmlTemplate(HomeTemplate)
}

#[derive(Template)]
#[template(path = "account.html")]
pub struct AccountTemplate {
    pub username: String,
    pub devices: Vec<DeviceView>,
}

/// View model for a device row on the account page.
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub last_synced_at: Option<String>,
    pub stale: bool,
}

#[derive(Template)]
#[template(path = "login_success.html")]
pub struct LoginSuccessTemplate;

#[derive(Template)]
#[template(path = "login_token.html")]
pub struct LoginTokenTemplate {
    pub token: String,
    pub device_id: String,
}
