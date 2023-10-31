use std::sync::Arc;

use axum::extract::State;
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use hyper::Method;
use hyper::StatusCode;
use tracing::warn;

use crate::{
    utils::{app_error::AppError, token::decode_token},
    AppState,
};

pub async fn a2f_login_route(
    method: Method,
    State(app_state): State<Arc<AppState>>,
    cookies: CookieJar,
    body: String,
) -> Result<(CookieJar, StatusCode), AppError> {
    let a2f_token = body;
    if a2f_token.is_empty() {
        warn!("{} /login/a2f Token missing", method);
        return Err(AppError::TokenMissing);
    }
    let a2f_token = match urlencoding::decode(&a2f_token) {
        Ok(token) => token,
        Err(e) => {
            warn!("{} /login/a2f Error while decoding token : {}", method, e);
            return Err(AppError::InvalidToken);
        }
    }
    .to_string();

    let token = decode_token(
        &a2f_token,
        &app_state.cipher,
        &format!("{} /login/a2f", method),
    )?;

    Ok((cookies.add(Cookie::new("session", token)), StatusCode::OK))
}
