use std::sync::Arc;

use axum::extract::State;
use chrono::Duration;
use hyper::Method;
use tracing::warn;

use crate::extractors::auth_extractor::InnerAuthUser;
use crate::{
    utils::{
        app_error::AppError,
        token::{create_token, decode_token},
    },
    AppState,
};

pub async fn email_confirm_route(
    method: Method,
    State(app_state): State<Arc<AppState>>,
    body: String,
) -> Result<String, AppError> {
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

    //Check if the email is already used
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

    let auth_user = match sqlx::query_as!(InnerAuthUser, r#"SELECT id, username, TRUE AS "email_verified!" FROM users"#).fetch_one(&app_state.pool).await {
        Ok(user) => user,
        Err(e) => {
            warn!("{}", e);
            return Err(AppError::InternalServerError);
        }
    };

    let auth_user = match serde_json::to_string(&auth_user) {
        Ok(user) => user,
        Err(e) => {
            warn!("{}", e);
            return Err(AppError::InternalServerError);
        }
    };

    let token = create_token(
        auth_user,
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
            Ok(token)
        },
        Err(e) => {
            warn!(
                "{} /register/email_confirm Error while verifying account : {}",
                method, e
            );
            Err(AppError::InternalServerError)
        }
    }
}
