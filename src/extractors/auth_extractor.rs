use axum::{extract::FromRequestParts, async_trait, http::request::Parts, response::{Response, IntoResponse}};
use tower_cookies::Cookies;
use serde::Deserialize;
use tracing::warn;

use crate::{utils::app_error::AppError, AppState};

#[derive(Deserialize)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub token: String,
    pub email_verified: bool,
}

pub struct AuthExtractor;

#[async_trait]
impl FromRequestParts<AppState> for Option<AuthUser> {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let cookies = match Cookies::from_request_parts(parts, state).await {
            Ok(cookies) => cookies,
            Err(e) => return Err(AppError::InternalServerError)
        };
        let token = match cookies.get("session") {
            Some(token) => token,
            None => return Ok(None)
        }.to_string();
        match sqlx::query_as!(AuthUser, "SELECT id, username, token, email_verified FROM users WHERE token = $1", token).fetch_optional(&state.pool).await {
            Ok(user) => {
                if let Some(inner_user) = user {
                    if !inner_user.email_verified {
                        Err(AppError::EmailNotConfirmed)
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            },
            Err(e) => {
                warn!("{e}");
                Err(AppError::InternalServerError)
            }
        }
    }
}
