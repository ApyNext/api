use std::sync::Arc;

use axum::extract::State;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use hyper::StatusCode;
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
    if body.is_empty() {
        warn!("Token missing");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Token de vérification d'email manquant."),
        ));
    }
    let a2f_token = urlencoding::decode(&body)
        .map_err(|e| {
            warn!("Error URL decoding token : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Token invalide."))
        })?
        .to_string();

    let token = Token::decode(&a2f_token, &app_state.cipher)?;

    Ok(cookies.add(Cookie::new("session", token)))
}
