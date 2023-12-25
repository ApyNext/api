use std::net::SocketAddr;

use axum::{body::Body, extract::ConnectInfo, http::Request, middleware::Next, response::Response};
use hyper::Method;
use tracing::info;

pub async fn logger(
    method: Method,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next<Body>,
) -> Response {
    info!("{addr} {method} {}", request.uri());

    next.run(request).await
}
