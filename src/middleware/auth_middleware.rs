use axum::{middleware::Next, response::Response};
use hyper::Request;
use tower_cookies::Cookies;

use crate::utils::app_error::AppError;


pub async fn auth_middleware<B>(
    cookies: Cookies,
    request: Request<B>,
    next: Next<B>
 ) -> Result<Response, AppError> {
    let token = match cookies.get("session") {
        Some(token) => token,
        None => return Err(AppError::InternalServerError)
    };

    Ok(next.run(request).await)
}
