use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Extension,
};
use hyper::StatusCode;
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::AuthUser, utils::app_error::AppError, AppState, SubscribedUsers,
    Subscriber, Users,
};

use super::ws_route::add_subscription;

pub struct Count {
    total: i64,
}

pub async fn follow_user_route(
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Extension(subscribed_users): Extension<SubscribedUsers>,
    Path(user_id): Path<i64>,
    State(app_state): State<Arc<AppState>>,
) -> Result<(), AppError> {
    let Some(auth_user) = auth_user else {
        warn!("Not connected");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Tu dois être connecté pour effectuer cette action."),
        ));
    };

    if auth_user.id == user_id {
        warn!("{} tried to follow himself", auth_user.id);
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

    for connection in &user.connections {
        let subscriber = Subscriber::new(connection.clone(), user.following.clone());
        add_subscription(auth_user.id, Arc::new(subscriber), subscribed_users.clone()).await;
    }

    Ok(())
}
