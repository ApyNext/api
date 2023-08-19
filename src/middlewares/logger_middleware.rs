use axum::{body::Body, http::Request, middleware::Next, response::Response};
use hyper::{HeaderMap, Method};
use shuttle_runtime::tracing::info;

pub async fn logger_middleware(
    method: Method,
    headers: HeaderMap,
    request: Request<Body>,
    next: Next<Body>,
    // ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    // info!("{} {}", request.uri(), addr);
    info!("{} {}", method, request.uri());
    info!("{:?}", headers.get("X-Forwarded-For").unwrap());
    // let body = match to_bytes(request.body_mut()).await {
    //     Ok(body) => body,
    //     Err(e) => {
    //         print!("{}", request.uri());
    //         return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    //     }
    // };

    // match String::from_utf8(body.to_vec()) {
    //     Ok(body) => info!("{} {}", request.uri(), body),
    //     Err(e) => info!("{} {:x}", request.uri(), body),
    // };
    next.run(request).await
}
