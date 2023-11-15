use axum::response::{IntoResponse, Response};

use crate::{extractors::auth_extractor::AuthUser, utils::app_error::AppError};

//TODO implement follow user route
pub async fn follow_user_route(
    AuthUser(auth_user): AuthUser,
    /*Extension(users): Extension<Users>,
    Path(username): Path<String>,
    State(app_state): State<Arc<AppState>>,*/
) -> Result<Response, AppError> {
    let _auth_user = match auth_user.as_ref() {
        Some(user) => user,
        None => return Err(AppError::YouHaveToBeConnectedToPerformThisAction),
    };

    Ok("".into_response())
}
