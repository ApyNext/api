use axum::extract::State;
use chrono::Duration;
use hyper::Method;
use hyper::StatusCode;
use rand::{
    distributions::{Alphanumeric, DistString},
    rngs::OsRng,
};
use tower_cookies::{Cookie, Cookies};
use tracing::warn;

use crate::{
    utils::{
        app_error::AppError,
        token::{create_token, decode_token},
    },
    AppState,
};

pub async fn email_confirm_route(
    method: Method,
    State(app_state): State<AppState>,
    cookies: Cookies,
    body: String,
) -> Result<StatusCode, AppError> {
    let email_verification_token = body;
    if email_verification_token.is_empty() {
        warn!("{} /register/email_confirm Token missing", method);
        return Err(AppError::TokenMissing);
    }
    let email_verification_token = match urlencoding::decode(&email_verification_token) {
        Ok(token) => token,
        Err(e) => {
            warn!(
                "{} /register/email_confirm Error while decoding token : {}",
                method, e
            );
            return Err(AppError::InvalidToken);
        }
    }
    .to_string();

    let email = decode_token(
        &email_verification_token,
        &app_state.cipher,
        &format!("{} /register/email_confirm", method),
    )?;

    //Check if email is already used
    match match sqlx::query!("SELECT id FROM users WHERE email = $1", email)
        .fetch_optional(&app_state.pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!("{} /register/email_confirm Error while checking if email address already exists : {}", method, e);
            return Err(AppError::InternalServerError);
        }
    } {
        Some(_) => {
            warn!(
                "{} /register/email_confirm Email address `{}` already used",
                method, email
            );
            return Err(AppError::EmailAddressAlreadyUsed);
        }
        None => (),
    };

    let token = create_token(
        Alphanumeric.sample_string(&mut OsRng, 256),
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
        Ok(_) => {
            cookies.add(Cookie::new("session", token));
            Ok(StatusCode::OK)
        }
        Err(e) => {
            warn!(
                "{} /register/email_confirm Error while verifying account : {}",
                method, e
            );
            Err(AppError::InternalServerError)
        }
    }
}
