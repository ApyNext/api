use axum::{body::Body, http::Request, middleware::Next, response::Response};
use hyper::Method;
use tracing::info;

pub async fn logger_middleware(
    method: Method,
    request: Request<Body>,
    next: Next<Body>,
) -> Response {
    info!("{} {}", method, request.uri());

    next.run(request).await
}
