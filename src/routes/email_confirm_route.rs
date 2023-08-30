use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use chrono::Duration;
use hyper::{Method, StatusCode};
use rand::{
    distributions::{Alphanumeric, DistString},
    rngs::OsRng,
};
use serde::Deserialize;
use shuttle_runtime::tracing::warn;
use crate::utils::register::DecodeTokenErrorKind;

use crate::{
    utils::token::{create_token, decode_token},
    AppState,
};

#[derive(Deserialize, Default)]
pub struct Token {
    token: String,
}

pub async fn email_confirm_route(
    method: Method,
    query: Option<Query<Token>>,
    State(app_state): State<AppState>,
) -> Response {
    let Query(email_verification_token) = query.unwrap_or_default();
    let email_verification_token = email_verification_token.token;
    if email_verification_token.is_empty() {
        warn!("{} /register/email_confirm Token missing", method);
        return (StatusCode::FORBIDDEN, "Token manquant").into_response();
    }
    let email_verification_token = match urlencoding::decode(&email_verification_token) {
        Ok(token) => token,
        Err(e) => {
            warn!("{} /register/email_confirm Error while decoding token : {}", method, e);
            return (StatusCode::FORBIDDEN, "Token invalide").into_response();
        }
    }.to_string();

    let email = match decode_token(&email_verification_token, &app_state.cipher) {
        Ok(email) => email,
        Err(DecodeTokenErrorKind::InvalidToken(e)) => {
            warn!(
                "{} /register/email_confirm {}",
                method,
                e
            );
            return (StatusCode::FORBIDDEN, "Lien invalide").into_response();
        },
        Err(DecodeTokenErrorKind::ExpiredToken) => {
            warn!(
                "{} /register/email_confirm Expired token",
                method
            );
            return (StatusCode::FORBIDDEN, "Lien d'activation expiré").into_response();
        }
    };
    let token = match create_token(
        Alphanumeric.sample_string(&mut OsRng, 256),
        Duration::days(365),
        &app_state.cipher,
    ) {
        Ok(token) => token,
        Err(e) => {
            warn!("{} /register/email_confirm {}", method, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    match sqlx::query!(
        "UPDATE users SET email = $1, email_verified = TRUE, token = $2 WHERE email = $3;",
        email,
        token,
        email_verification_token
    )
    .execute(&app_state.pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            warn!(
                "{} /register/email_confirm Error while verifying account : {}",
                method, e
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    token.into_response()
}
