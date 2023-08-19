use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use hyper::StatusCode;
use sha2::{Digest, Sha512};
use shuttle_runtime::tracing::info;
use time::OffsetDateTime;

use crate::{structs::register_user::RegisterUser, utils::register::generate_token, AppState};

pub async fn register_route(
    State(app_state): State<AppState>,
    Json(register_user): Json<RegisterUser>,
) -> Response {
    let mut hasher = Sha512::new();
    hasher.update(register_user.password);
    let password = format!("{:x}", hasher.finalize());
    let token = generate_token();
    let birthdate = match OffsetDateTime::from_unix_timestamp(register_user.birthdate) {
        Ok(birthdate) => birthdate,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    match sqlx::query!("INSERT INTO users (username, email, password, birthdate, biography, is_male, token) VALUES ($1, $2, $3, $4, $5, $6, $7);", register_user.username, register_user.email, password, birthdate, register_user.biography, register_user.is_male, token).execute(&app_state.pool).await {
        Ok(_) => "ok".into_response(),
        Err(e) => {
            info!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }
}
