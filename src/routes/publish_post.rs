use std::sync::Arc;

use crate::{
    extractors::auth_extractor::AuthUser, structs::post::Post, utils::app_error::AppError, AppState,
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Serialize, Deserialize)]
pub struct PublishPost {
    title: String,
    content: String,
}

pub async fn publish_post_route(
    State(app_state): State<Arc<AppState>>,
    AuthUser(auth_user): AuthUser,
    Json(post): Json<PublishPost>,
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

    if let Err(e) = sqlx::query_as!(
        Post,
        r#"INSERT INTO post (author, title, content) VALUES ($1, $2, $3) RETURNING *"#,
        auth_user.id,
        post.title,
        post.content
    )
    .fetch_one(&app_state.pool)
    .await
    {
        warn!("Error inserting post with author {} : {e}", auth_user.id);
        return Err(AppError::internal_server_error());
    };

    Ok("".to_string())
}
