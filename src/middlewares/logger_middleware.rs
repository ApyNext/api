use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use hyper::{body::to_bytes, StatusCode};
use shuttle_runtime::tracing::info;

pub async fn logger_middleware(
    mut request: Request<Body>,
    next: Next<Body>,
    // ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    // info!("{} {}", request.uri(), addr);
    let body = match to_bytes(request.body_mut()).await {
        Ok(body) => body,
        Err(e) => {
            print!("{}", request.uri());
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    match String::from_utf8(body.to_vec()) {
        Ok(body) => info!("{} {}", request.uri(), body),
        Err(to_string) => info!("{} {:x}", request.uri(), body),
    };
    next.run(request).await
}
