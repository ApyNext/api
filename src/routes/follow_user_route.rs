use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Extension,
};
use tracing::info;

use crate::{
    extractors::auth_extractor::AuthUser,
    routes::sse::{broadcast_msg, Message},
    utils::app_error::AppError,
    AppState, Users,
};

pub async fn follow_user_route(
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Path(username): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Response, AppError> {
    let auth_user = match auth_user.as_ref() {
        Some(user) => user,
        None => return Err(AppError::YouHaveToBeConnectedToPerformThisAction),
    };

    Ok("".into_response())
}
