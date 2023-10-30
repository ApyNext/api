use axum::response::{Response, IntoResponse};
use tracing::info;

use crate::{utils::app_error::AppError, extractors::auth_extractor::AuthUser};

pub async fn follow_user_route(auth_user: Option<AuthUser>) -> Result<Response, AppError> {
    let auth_user = match auth_user {
        Some(user) => user,
        None => return Err(AppError::YouHaveToBeConnectedToPerformThisAction)
    };
    info!("{}", auth_user.username);
    Ok("".into_response())
}
