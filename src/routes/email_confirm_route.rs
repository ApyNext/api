use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
};
use chrono::Duration;
use hyper::{Method, StatusCode};
use serde::Deserialize;
use shuttle_runtime::tracing::warn;

use crate::{
    utils::jwt::{create_jwt, decode_email_jwt, decode_refresh_jwt},
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
    token: String,
}

pub async fn email_confirm_route(
    method: Method,
    query: Option<Query<Token>>,
    State(app_state): State<AppState>,
) -> Response {
    let Query(token) = query.unwrap_or_default();
    let token = token.token;
    let email = match decode_email_jwt(&token, app_state.secret_key.as_bytes()) {
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
    let email_confirm_user = match sqlx::query_as!(
        EmailConfirmUser,
        "UPDATE users SET email = $1, email_verified = TRUE WHERE email = $2 RETURNING token, id;",
        email,
        token
    )
    .fetch_one(&app_state.pool)
    .await
    {
        Ok(token) => token,
        Err(e) => {
            warn!(
                "{} /register/email_confirm Error while verifying account : {}",
                method, e
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    match decode_refresh_jwt(&email_confirm_user.token, app_state.secret_key.as_bytes()) {
        Ok(_) => (),
        Err(res) => {
            warn!(
                "{} /register/email_confirm Error while decoding refresh JWT : {}",
                method,
                res.status()
            );
            return res;
        }
    }
    match create_jwt(
        Some(email_confirm_user.id.to_string()),
        app_state.secret_key.as_bytes(),
        Duration::minutes(15),
    ) {
        Ok(jwt) => jwt.into_response(),
        Err(e) => {
            warn!("{} /register/email_confirm {}", method, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}
