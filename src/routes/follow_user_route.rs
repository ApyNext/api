use axum::{response::{Response, IntoResponse}, Extension, extract::Path};
use tracing::info;

use crate::{utils::app_error::AppError, extractors::auth_extractor::AuthUser, routes::sse::{broadcast_msg, Message}, Users};

pub async fn follow_user_route(AuthUser(auth_user): AuthUser, Extension(users): Extension<Users>, Path(username): Path<String>) -> Result<Response, AppError> {
    let auth_user = match auth_user.as_ref() {
        Some(user) => user,
        None => return Err(AppError::YouHaveToBeConnectedToPerformThisAction)
    };

    let msg = Message {
        author: auth_user.id,
        content: format!("Hi ! I'm {} and I want to follow @{}", auth_user.username, username)
    };

    broadcast_msg(msg, users).await;
    info!("{}", auth_user.username);
    Ok("".into_response())
}
