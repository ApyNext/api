use std::sync::Arc;

use axum::extract::{Path, State};
use tracing::{info, warn};

use crate::{extractors::auth_extractor::AuthUser, utils::app_error::AppError, AppState};

pub struct Count {
    total: i64,
}

//TODO implement follow user route
pub async fn follow_user_route(
    AuthUser(auth_user): AuthUser,
    //Extension(users): Extension<Users>,
    Path(user_id): Path<i64>,
    State(app_state): State<Arc<AppState>>,
) -> Result<(), AppError> {
    let auth_user = match auth_user {
        Some(user) => user,
        None => return Err(AppError::YouHaveToBeConnectedToPerformThisAction),
    };

    if auth_user.id == user_id {
        info!("{} tried to follow himself", auth_user.id);
        return Err(AppError::YouCannotFollowYourself);
    }

    let count = match sqlx::query_as!(
        Count,
        r#"SELECT COUNT(*) as "total!" FROM follow WHERE follower_id = $1 AND followed_id = $2"#,
        auth_user.id,
        user_id
    )
    .fetch_one(&app_state.pool)
    .await
    {
        Ok(count) => count,
        Err(e) => {
            warn!("{e}");
            return Err(AppError::InternalServerError);
        }
    };

    if count.total != 0 {
        info!("{} already following {}", auth_user.id, user_id);
        return Err(AppError::YouAreAlreadyFollowingThisUser);
    }

    match sqlx::query!(
        "INSERT INTO follow (follower_id, followed_id) VALUES ($1, $2)",
        auth_user.id,
        user_id
    )
    .execute(&app_state.pool)
    .await
    {
        Ok(_) => {}
        Err(e) => {
            warn!("{e}");
            return Err(AppError::InternalServerError);
        }
    }

    Ok(())
}
