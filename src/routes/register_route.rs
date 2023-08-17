use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use sha2::{Digest, Sha512};
use sqlx::PgPool;

use crate::structs::register_user::RegisterUser;

pub async fn register_route(
    State(pool): State<PgPool>,
    Json(mut register_user): Json<RegisterUser>,
) -> Response {
    let mut hasher = Sha512::new();
    hasher.update(register_user.password);
    register_user.password = format!("{:x}", hasher.finalize());
    let account = sqlx::query_as!(User, "INSERT INTO users (username, email, password, birthdate, biography) VALUES ($1, $2, $3, $4, $5);", register_user.username, register_user.email, register_user.password, register_user.birthdate, register_user.biography);
    "In development".to_string().into_response()
}
