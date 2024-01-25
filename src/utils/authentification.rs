use std::sync::Arc;

use tracing::warn;

use crate::{
    extractors::auth_extractor::{AuthUser, InnerAuthUser},
    AppState,
};

use super::app_error::AppError;

pub async fn authentificate(app_state: Arc<AppState>, token: &str) -> Result<AuthUser, AppError> {
    let token = match urlencoding::decode(token) {
        Ok(token) => token,
        Err(e) => {
            warn!("{e}");
            return Ok(AuthUser(None));
        }
    }
    .to_string();
    if let Some(user) = sqlx::query_as!(
        InnerAuthUser,
        "SELECT id FROM account WHERE token = $1 AND email_verified = TRUE",
        token
    )
    .fetch_optional(&app_state.pool)
    .await
    .map_err(|e| {
        warn!("Error getting auth user from database : {e}");
        AppError::internal_server_error()
    })? {
        Ok(AuthUser(Some(user)))
    } else {
        Ok(AuthUser(None))
    }
}
