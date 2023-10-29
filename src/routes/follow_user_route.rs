use axum::response::{Response, IntoResponse};

use crate::utils::app_error::AppError;

pub async fn follow_user_route() -> Result<Response, AppError> {
    //TODO
    Ok("".into_response())
}
