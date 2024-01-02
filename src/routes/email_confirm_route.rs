use std::sync::Arc;

use axum::extract::State;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use chrono::Duration;
use hyper::StatusCode;
use rand::distributions::{Alphanumeric, DistString};
use rand::thread_rng;
use tracing::warn;

use crate::utils::token::Token;
use crate::{utils::app_error::AppError, AppState};

pub async fn email_confirm_route(
    State(app_state): State<Arc<AppState>>,
    cookies: CookieJar,
    body: String,
) -> Result<CookieJar, AppError> {
    if body.is_empty() {
        warn!("Token missing");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Token de confirmation d'email manquant."),
        ));
    }
    let email_verification_token = urlencoding::decode(&body)
        .map_err(|e| {
            warn!("Error URL decoding email confirmation token : {e}");
            AppError::new(
                StatusCode::FORBIDDEN,
                Some("Token de confirmation d'email invalide."),
            )
        })?
        .to_string();

    let email = Token::decode(&email_verification_token, &app_state.cipher)?;

    //Check if the email is already used
    let result = sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&app_state.pool)
        .await
        .map_err(|e| {
            warn!("Error checking if the email `{email}` already exists in the database : {e}");
            AppError::internal_server_error()
        })?;
    if result.is_some() {
        warn!("Email address `{email}` already used");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Email déjà utilisé."),
        ));
    };

    let token = Token::create(
        Alphanumeric.sample_string(&mut thread_rng(), 256),
        Duration::days(365),
        &app_state.cipher,
    );

    sqlx::query!(
        "UPDATE users SET email = $1, email_verified = TRUE, token = $2 WHERE email = $3;",
        email,
        token,
        email_verification_token
    )
    .execute(&app_state.pool)
    .await
    .map_err(|e| {
        warn!("Error validating account : {}", e);
        AppError::internal_server_error()
    })?;

    Ok(cookies.add(Cookie::new("session", token)))
}
