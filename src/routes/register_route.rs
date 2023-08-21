use axum::response::Response;
use axum::{extract::State, response::IntoResponse, Json};
use hyper::Method;
use hyper::StatusCode;
use sha2::{Digest, Sha512};
use shuttle_runtime::tracing::warn;
use time::OffsetDateTime;

use crate::utils::register::check_register_infos;
use crate::utils::register::create_email_jwt;
use crate::{
    structs::{register_user::RegisterUser, user::User},
    utils::register::send_html_message,
    AppState,
};

pub async fn register_route(
    method: Method,
    State(app_state): State<AppState>,
    Json(mut register_user): Json<RegisterUser>,
) -> Response {
    register_user.username = register_user.username.to_lowercase();
    register_user.email = register_user.email.to_lowercase();
    check_register_infos(&register_user).map_err(|error| return error);
    let mut hasher = Sha512::new();
    hasher.update(register_user.password);
    let password = format!("{:x}", hasher.finalize());
    let birthdate = match OffsetDateTime::from_unix_timestamp(register_user.birthdate) {
        Ok(birthdate) => birthdate,
        Err(e) => {
            warn!("{}", e);
            return (StatusCode::FORBIDDEN, "Date de naissance invalide").into_response();
        }
    };

    match match sqlx::query!("SELECT * FROM users WHERE email = $1", register_user.email)
        .fetch_optional(&app_state.pool)
        .await
    {
        Ok(result) => result,
        Err(e) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    } {
        Some(_) => return (StatusCode::FORBIDDEN, "Adresse email déjà utilisée").into_response(),
        None => (),
    };

    let account = match sqlx::query_as!(User, "INSERT INTO users (username, email, password, birthdate, biography, is_male, token) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *;", register_user.username, register_user.email, password, birthdate, register_user.biography, register_user.is_male, "").fetch_one(&app_state.pool).await {
        Ok(account) => account,
        Err(e) => return e.to_string().into_response()
    };

    println!("{:?}", account);

    let email_confirm_token = match create_email_jwt(register_user.email.clone(), "test".as_bytes())
    {
        Ok(jwt) => jwt,
        Err(code) => return code.into_response(),
    };

    match send_html_message(
        app_state.smtp_client,
        "Confirm email",
        &format!("<h1>Token : {}</h1>", email_confirm_token),
        register_user.email.parse().unwrap(),
    ) {
        Ok(_) => (),
        Err(e) => {
            warn!("{} `{}`", method, e.to_string());
        }
    };

    // create_jwt(user_infos, "secret_key".as_bytes()).map_err(|e| e)
    "ok".into_response()
}
