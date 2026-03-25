use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
};
use governor::{clock::DefaultClock, Quota, RateLimiter};
use std::{
    net::{IpAddr, SocketAddr},
    num::NonZeroU32,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};

/// Shared rate limiter state (not keyed — one limiter per layer instance, but we
/// use per-IP limiters via a DashMap wrapper).
type IpRateLimiter =
    governor::RateLimiter<IpAddr, governor::state::keyed::DashMapStateStore<IpAddr>, DefaultClock>;

/// Configuration for rate limiting.
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed in the window.
    pub requests: u32,
    /// Window duration in seconds.
    pub window_secs: u64,
}

impl RateLimitConfig {
    /// Standard rate limit: 120 requests per minute per IP.
    pub fn standard() -> Self {
        Self {
            requests: 120,
            window_secs: 60,
        }
    }

    /// Strict rate limit: 30 requests per minute per IP.
    pub fn strict() -> Self {
        Self {
            requests: 30,
            window_secs: 60,
        }
    }
}

/// A Tower layer that applies IP-based rate limiting.
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<IpRateLimiter>,
}

impl RateLimitLayer {
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::with_period(std::time::Duration::from_secs(config.window_secs))
            .expect("window_secs must be > 0")
            .allow_burst(NonZeroU32::new(config.requests).expect("requests must be > 0"));

        let limiter = Arc::new(RateLimiter::dashmap(quota));
        Self { limiter }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

/// The middleware service that checks the rate limiter before forwarding the request.
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: Arc<IpRateLimiter>,
}

/// Extract client IP from the request using `ConnectInfo<SocketAddr>` (actual TCP peer).
///
/// We intentionally ignore `X-Forwarded-For` because it is client-controlled and
/// trivially spoofable. If this server is deployed behind a trusted reverse proxy,
/// this function should be updated to read the proxy-appended IP instead.
fn extract_client_ip(req: &Request<Body>) -> IpAddr {
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip();
    }

    // Fallback — should not happen when using into_make_service_with_connect_info
    tracing::warn!("Could not determine client IP, falling back to 0.0.0.0");
    IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
}

impl<S> Service<Request<Body>> for RateLimitService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let ip = extract_client_ip(&req);
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match limiter.check_key(&ip) {
                Ok(_) => inner.call(req).await,
                Err(not_until) => {
                    let retry_after = not_until.wait_time_from(governor::clock::Clock::now(
                        &governor::clock::DefaultClock::default(),
                    ));
                    let retry_secs = retry_after.as_secs().max(1);

                    tracing::warn!(
                        ip = %ip,
                        retry_after_secs = retry_secs,
                        "Rate limit exceeded"
                    );

                    let response = (
                        StatusCode::TOO_MANY_REQUESTS,
                        [("retry-after", retry_secs.to_string())],
                        "Too many requests. Please try again later.",
                    )
                        .into_response();

                    Ok(response)
                }
            }
        })
    }
}
