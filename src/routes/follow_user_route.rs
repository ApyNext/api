use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Extension,
};
use hyper::StatusCode;
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::AuthUser,
    utils::{
        app_error::AppError,
        real_time_event_management::{EventTracker, RealTimeEvent},
        register::Record,
    },
    AppState, Users,
};

pub struct Count {
    total: i64,
}

pub async fn follow_user_route(
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Extension(event_tracker): Extension<EventTracker>,
    Path(user_username): Path<String>,
    State(app_state): State<Arc<AppState>>,
) -> Result<(), AppError> {
    let Some(auth_user) = auth_user else {
        warn!("Not connected");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Tu dois être connecté pour effectuer cette action."),
        ));
    };

    let user = sqlx::query_as!(
        Record,
        r#"SELECT id FROM users WHERE username = $1"#,
        user_username
    )
    .fetch_optional(&app_state.pool)
    .await
    .map_err(|e| {
        warn!("Error getting id of user {user_username} : {e}");
        AppError::internal_server_error()
    })?;

    let Some(user) = user else {
        warn!("Cannot follow user `{user_username}` that doesn't exist");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some(format!("L'utilisateur {user_username} n'existe pas.")),
        ));
    };

    let user_id = user.id;

    if auth_user.id == user_id {
        warn!(
            "{} with id {} tried to follow himself",
            user_username, auth_user.id
        );
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Tu ne peux pas te suivre toi-même."),
        ));
    }

    let count = sqlx::query_as!(
        Count,
        r#"SELECT COUNT(*) as "total!" FROM follow WHERE follower_id = $1 AND followed_id = $2"#,
        auth_user.id,
        user_id
    )
    .fetch_one(&app_state.pool)
    .await
    .map_err(|e| {
        warn!(
            "Error checking follow from `{}` to `{}` : {}",
            auth_user.id, user_id, e
        );
        AppError::internal_server_error()
    })?;

    if count.total != 0 {
        info!("{} already following {}", auth_user.id, user_id);
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Tu suis déjà cet utilisateur."),
        ));
    }

    sqlx::query!(
        "INSERT INTO follow (follower_id, followed_id) VALUES ($1, $2)",
        auth_user.id,
        user_id
    )
    .execute(&app_state.pool)
    .await
    .map_err(|e| {
        warn!(
            "Error creating follow from `{}` to `{}` : {}",
            auth_user.id, user_id, e
        );
        AppError::internal_server_error()
    })?;

    let mut writer = users.write().await;

    let Some(user) = writer.get_mut(&auth_user.id) else {
        return Ok(());
    };

    //TODO Not sure if that's a good idea...
    for connection in user.clone() {
        event_tracker
            .subscribe(
                RealTimeEvent::NewPostNotification {
                    followed_user_id: user_id,
                },
                connection,
            )
            .await;
    }

    Ok(())
}
