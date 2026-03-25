use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// CSRF protection middleware.
///
/// Requires a custom `X-Requested-With` header on all mutation requests
/// (POST, PUT, PATCH, DELETE) that use session-cookie authentication.
/// Browsers cannot send custom headers cross-origin without a CORS preflight,
/// so this effectively blocks CSRF attacks from third-party sites.
///
/// Requests are allowed through without the header if:
/// - The HTTP method is safe (GET, HEAD, OPTIONS)
/// - The request carries a `Bearer` token in the `Authorization` header
///   (API/CLI clients are not vulnerable to CSRF)
pub async fn csrf_protection(request: Request<Body>, next: Next) -> Response {
    let method = request.method().clone();

    // Safe methods are always allowed through.
    if method == Method::GET || method == Method::HEAD || method == Method::OPTIONS {
        return next.run(request).await;
    }

    // Bearer-token requests are not vulnerable to CSRF — skip the check.
    let has_bearer = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.starts_with("Bearer "));

    if has_bearer {
        return next.run(request).await;
    }

    // For session-authenticated mutation requests, require the custom header.
    let has_csrf_header = request.headers().contains_key("x-requested-with");

    if !has_csrf_header {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({"error": "CSRF validation failed"})),
        )
            .into_response();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, middleware, routing::post, Router};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn test_app() -> Router {
        Router::new()
            .route("/mutate", post(ok_handler))
            .layer(middleware::from_fn(csrf_protection))
    }

    #[tokio::test]
    async fn allows_get_without_header() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/mutate")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // GET on a POST-only route returns 405, not 403 — meaning CSRF allowed it through.
        assert_ne!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn blocks_post_without_header() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/mutate")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "CSRF validation failed");
    }

    #[tokio::test]
    async fn allows_post_with_csrf_header() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/mutate")
            .header("x-requested-with", "XMLHttpRequest")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn allows_post_with_bearer_token() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/mutate")
            .header("authorization", "Bearer rk_test_token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Should pass CSRF check (200), not 403.
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
