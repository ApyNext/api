use std::sync::Arc;

use crate::models::account::AccountPermission;
use crate::models::post::{PublicPost, PublicPostAuthor};
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

    if post.title.len() < 3 || post.title.len() > 50 {
        warn!(
            "User {} tried to create a post with a title with a wrong length : {}/50",
            auth_user.id,
            post.title.len()
        );
        return Err(AppError::forbidden_error(Some(
            "Le titre d'un post doit contenir entre 3 et 50 caractères.",
        )));
    }

    if post.content.len() < 10 || post.content.len() > 1000 {
        warn!(
            "User {} tried to create a post with a content with a wrong length : {}/1000",
            auth_user.id,
            post.content.len()
        );
        return Err(AppError::forbidden_error(Some(
            "Le contenu d'un post doit contenir entre 10 et 1 000 caractères.",
        )));
    }

    struct PostWithAuthorWrong {
        id: i64,
        title: String,
        content: String,
        created_at: OffsetDateTime,
        updated_at: OffsetDateTime,
        author_id: i64,
        author_username: String,
        author_permission: AccountPermission,
    }

    let post = match sqlx::query_file_as!(
        PostWithAuthorWrong,
        "./src/queries/insert_post.sql",
        auth_user.id,
        post.title,
        post.content,
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

    let post = PublicPost {
        id: post.id,
        title: post.title,
        content: post.content,
        author: PublicPostAuthor {
            id: post.author_id,
            username: post.author_username,
            permission: post.author_permission,
        },
        created_at: post.created_at,
        updated_at: post.updated_at,
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
