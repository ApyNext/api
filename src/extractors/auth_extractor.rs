use std::sync::Arc;

use axum::{extract::{FromRequestParts, FromRef}, async_trait, http::request::Parts};
use axum_extra::extract::CookieJar;
use serde::{Serialize, Deserialize};
use tracing::warn;

use crate::{utils::{app_error::AppError, token::decode_token}, AppState};

#[derive(Serialize, Deserialize)]
pub struct InnerAuthUser {
    pub id: i64,
    pub username: String,
    pub email_verified: bool,
}

pub struct AuthUser(pub Option<Arc<InnerAuthUser>>);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);
        let cookies = CookieJar::from_request_parts(parts, state).await.unwrap();
        let token = match cookies.get("session") {
            Some(token) => token,
            None => return Ok(AuthUser(None))
        }.to_string();
        let token = decode_token(&token, &app_state.cipher, "Auth extractor")?;
        match serde_json::from_str::<InnerAuthUser>(&token) {
            Ok(token) => Ok(AuthUser(Some(Arc::new(token)))),
            Err(e) => {
                warn!("{}", e);
                Err(AppError::InternalServerError)
            }
        }
    }
}
