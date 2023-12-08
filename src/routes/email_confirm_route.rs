use std::sync::Arc;

use axum::extract::State;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use chrono::Duration;
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
        return Err(AppError::TokenMissing);
    }
    let email_verification_token = match urlencoding::decode(&body) {
        Ok(token) => token,
        Err(e) => {
            warn!("Error while decoding token : {}", e);
            return Err(AppError::InvalidToken);
        }
    }
    .to_string();

    let email = Token::decode(&email_verification_token, &app_state.cipher)?;

    //Check if the email is already used
    if match sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&app_state.pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!(
                "Error while checking if email address already exists : {}",
                e
            );
            return Err(AppError::InternalServerError);
        }
    }
    .is_some()
    {
        warn!("Email address `{}` already used", email);
        return Err(AppError::EmailAddressAlreadyUsed);
    };

    let token = Token::new(
        Alphanumeric.sample_string(&mut thread_rng(), 256),
        Duration::days(365),
        &app_state.cipher,
    );

    match sqlx::query!(
        "UPDATE users SET email = $1, email_verified = TRUE, token = $2 WHERE email = $3;",
        email,
        token,
        email_verification_token
    )
    .execute(&app_state.pool)
    .await
    {
        Ok(_) => Ok(cookies.add(Cookie::new("session", token))),
        Err(e) => {
            warn!("Error while verifying account : {}", e);
            Err(AppError::InternalServerError)
        }
    }
}
