use axum::{extract::Query, response::IntoResponse};

use crate::{
    extractors::auth_extractor::AuthUser,
    structs::{pagination::PaginationParams, post::PublicPost},
    utils::app_error::AppError,
};

pub async fn get_posts_route(
    AuthUser(auth_user): AuthUser,
    Query(pagination_params): Query<PaginationParams>,
) -> Result<impl IntoResponse, AppError> {
    let offset = pagination_params.offset.unwrap_or(0);
    let limit = pagination_params.limit.unwrap_or(10);

    // let posts: Vec<PublicPost> = match sqlx::query_as!(PublicPost, "SELECT ");

    Ok("".to_string())
}
