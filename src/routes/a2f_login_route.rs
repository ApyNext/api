use std::sync::Arc;

use axum::extract::State;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use tracing::warn;

use crate::{
    utils::{app_error::AppError, token::Token},
    AppState,
};

pub async fn a2f_login_route(
    State(app_state): State<Arc<AppState>>,
    cookies: CookieJar,
    body: String,
) -> Result<CookieJar, AppError> {
    let a2f_token = body;
    if a2f_token.is_empty() {
        warn!("Token missing");
        return Err(AppError::TokenMissing);
    }
    let a2f_token = match urlencoding::decode(&a2f_token) {
        Ok(token) => token,
        Err(e) => {
            warn!("Error while decoding token : {}", e);
            return Err(AppError::InvalidToken);
        }
    }
    .to_string();

    let token = Token::decode(&a2f_token, &app_state.cipher)?;

    Ok(cookies.add(Cookie::new("session", token)))
}
