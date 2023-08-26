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

use crate::{
    utils::token::{create_token, decode_token},
    AppState,
};

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

impl Default for Token {
    fn default() -> Self {
        Self {
            token: "".to_string(),
        }
    }
}

pub struct EmailConfirmUser {
    id: i64,
}

pub async fn email_confirm_route(
    method: Method,
    query: Option<Query<Token>>,
    State(app_state): State<AppState>,
) -> Response {
    let Query(email_verification_token) = query.unwrap_or_default();
    let email_verification_token = email_verification_token.token;
    //TODO décommenter
    let email = match decode_token(&email_verification_token, &app_state.cipher) {
        Ok(email) => email,
        Err(res) => {
            warn!(
                "{} /register/email_confirm Error while decoding token : {}",
                method,
                res.status()
            );
            return res;
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
    let email_confirm_user = match sqlx::query!(
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
