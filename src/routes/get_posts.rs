use crate::{
    extractors::auth_extractor::AuthUser,
    models::{
        account::AccountPermission,
        post::{PublicPost, PublicPostAuthor},
    },
    utils::{app_error::AppError, pagination::PaginationParams},
    AppState,
};
use axum::extract::{Query, State};
use serde_json::json;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::warn;

pub async fn get_posts_route(
    AuthUser(auth_user): AuthUser,
    Query(pagination_params): Query<PaginationParams>,
    State(app_state): State<Arc<AppState>>,
) -> Result<String, AppError> {
    let mut limit = pagination_params.limit.unwrap_or(10);
    if limit == 0 {
        return Ok(String::new());
    }
    if limit.is_negative() {
        limit = 10;
    }

    let mut offset = pagination_params.offset.unwrap_or(0);
    if offset.is_negative() {
        offset = 0;
    }

    struct PublicPostWithAuthorWrong {
        pub id: i64,
        pub title: String,
        pub content: String,
        pub created_at: OffsetDateTime,
        pub updated_at: OffsetDateTime,
        pub author_id: i64,
        pub author_username: String,
        pub author_permission: AccountPermission,
    }

    let posts = sqlx::query_file_as!(
        PublicPostWithAuthorWrong,
        "./src/queries/select_posts.sql",
        limit,
        offset
    )
    .fetch_all(&app_state.pool)
    .await
    .map_err(|e| {
        warn!("Error getting posts : {e}");
        AppError::internal_server_error()
    })?;

    let posts: Vec<PublicPost> = posts
        .into_iter()
        .map(|post| PublicPost {
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
        })
        .collect();

    Ok(json! {posts}.to_string())
}
