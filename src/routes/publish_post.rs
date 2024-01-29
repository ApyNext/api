use std::sync::Arc;

use crate::models::account::AccountPermission;
use crate::models::post::{NotificationPost, PublicPostAuthor};
use crate::utils::post::check_new_post_data;
use crate::{
    extractors::auth_extractor::AuthUser,
    utils::{
        app_error::AppError,
        real_time_event_management::{EventTracker, RealTimeEvent, WsEvent},
    },
    AppState,
};
use axum::{extract::State, Extension, Json};
use time::OffsetDateTime;
use tracing::warn;

#[derive(serde::Deserialize)]
pub struct NewPost {
    pub title: String,
    pub content: String,
}

pub async fn publish_post_route(
    State(app_state): State<Arc<AppState>>,
    AuthUser(auth_user): AuthUser,
    Extension(event_tracker): Extension<EventTracker>,
    Json(post): Json<NewPost>,
) -> Result<String, AppError> {
    let Some(auth_user) = auth_user else {
        warn!("User not connected");
        return Err(AppError::you_have_to_be_connected_to_perform_this_action_error());
    };

    let title = post.title.trim();
    let content = post.content.trim();

    check_new_post_data(auth_user.id, title, content)?;

    struct PostWithAuthorWrong {
        id: i64,
        title: String,
        created_at: OffsetDateTime,
        author_id: i64,
        author_username: String,
        author_permission: AccountPermission,
    }

    let post = match sqlx::query_file_as!(
        PostWithAuthorWrong,
        "./src/queries/insert_post.sql",
        auth_user.id,
        title,
        content,
    )
    .fetch_one(&app_state.pool)
    .await
    {
        Ok(post) => post,
        Err(e) => {
            warn!("Error inserting post with author {} : {e}", auth_user.id);
            return Err(AppError::internal_server_error());
        }
    };

    let post = NotificationPost {
        id: post.id,
        title: post.title,
        author: PublicPostAuthor {
            id: post.author_id,
            username: post.author_username,
            permission: post.author_permission,
        },
        created_at: post.created_at,
    };

    let event = WsEvent::new_new_post_notification_event(&post);

    event_tracker
        .notify(
            RealTimeEvent::NewPostNotification {
                followed_user_id: auth_user.id,
            },
            event.to_string(),
        )
        .await;

    Ok(post.id.to_string())
}
