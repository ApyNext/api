use std::sync::Arc;

use crate::models::post::NewPost;
use crate::{
    extractors::auth_extractor::AuthUser,
    utils::{
        app_error::AppError,
        real_time_event_management::{EventTracker, RealTimeEvent, WsEvent},
    },
    AppState,
};
use axum::{extract::State, Extension, Json};
use tracing::warn;

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

    let post = match sqlx::query_as!(
        PublicPost,
        r#"WITH inserted_post AS (INSERT INTO post (author_id, title, content) VALUES ($1, $2, $3) RETURNING *) SELECT inserted_post.id, title, content, inserted_post.created_at, inserted_post.updated_at, account.id AS "author.id", account.username AS "author.username" FROM inserted_post JOIN account ON inserted_post.author_id = account.id"#,
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
        },
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
