use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use sha2::{Digest, Sha512};
use shuttle_runtime::tracing::info;
use sqlx::PgPool;

use crate::{structs::register_user::RegisterUser, utils::register::generate_token};

pub async fn register_route(
    State(pool): State<PgPool>,
    Json(mut register_user): Json<RegisterUser>,
) -> Response {
    let mut hasher = Sha512::new();
    hasher.update(register_user.password);
    register_user.password = format!("{:x}", hasher.finalize());
    let token = generate_token();
    match sqlx::query!("INSERT INTO users (username, email, password, birthdate, biography, is_male, token) VALUES ($1, $2, $3, $4, $5, $6, $7);", register_user.username, register_user.email, register_user.password, register_user.birthdate, register_user.biography, register_user.is_male, token).execute(&pool).await {
        Ok(_) => (),
        Err(e) => {
            info!("{e}");
            return "error".into_response();
        }
    };
    "ok".into_response()
}
