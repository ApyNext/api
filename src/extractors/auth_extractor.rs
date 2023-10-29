use axum::{extract::FromRequestParts, async_trait, http::request::Parts, response::{Response, IntoResponse}};
use tower_cookies::Cookies;
use serde::Deserialize;
use tracing::warn;

use crate::{utils::app_error::AppError, AppState};

#[derive(Deserialize)]
pub struct AuthUser {
    id: i64,
    username: String,
    token: String,
    email_verified: bool,
}

pub struct AuthExtractor;

#[async_trait]
impl FromRequestParts<AppState> for Option<AuthUser> {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let cookies = match Cookies::from_request_parts(parts, state).await {
            Ok(cookies) => cookies,
            Err(e) => return Err(e.into_response())
        };
        let token = match cookies.get("session") {
            Some(token) => token,
            None => return Ok(None)
        }.to_string();
        match sqlx::query_as!(AuthUser, "SELECT id, username, token, email_verified FROM users WHERE token = $1", token).fetch_optional(&state.pool).await {
            Ok(user) => Ok(user),
            Err(e) => {
                warn!("{e}");
                Err(AppError::InternalServerError.into_response())
            }
        }
    }
}
