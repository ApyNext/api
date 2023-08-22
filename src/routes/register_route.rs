use axum::response::Response;
use axum::{extract::State, response::IntoResponse, Json};
use chrono::Duration;
use hyper::Method;
use hyper::StatusCode;
use sha2::{Digest, Sha512};
use shuttle_runtime::tracing::warn;
use time::OffsetDateTime;

use crate::utils::jwt::create_jwt;
use crate::utils::register::check_register_infos;
use crate::API_URL;
use crate::{structs::register_user::RegisterUser, utils::register::send_html_message, AppState};

pub async fn register_route(
    method: Method,
    State(app_state): State<AppState>,
    Json(mut register_user): Json<RegisterUser>,
) -> Response {
    register_user.username = register_user.username.to_lowercase();
    register_user.email = register_user.email.to_lowercase();
    match check_register_infos(&register_user) {
        Ok(_) => (),
        Err(e) => {
            warn!("{} /register {}", method, e);
            return e.into_response();
        }
    }
    let mut hasher = Sha512::new();
    hasher.update(register_user.password);
    let password = format!("{:x}", hasher.finalize());
    let birthdate = match OffsetDateTime::from_unix_timestamp(register_user.birthdate) {
        Ok(birthdate) => birthdate,
        Err(e) => {
            warn!("{} /register Date de naissance invalide : {}", method, e);
            return (StatusCode::FORBIDDEN, "Date de naissance invalide").into_response();
        }
    };

    match match sqlx::query!("SELECT id FROM users WHERE email = $1", register_user.email)
        .fetch_optional(&app_state.pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!("{} /register {}", method, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    } {
        Some(_) => {
            warn!(
                "{} /register Adresse email `{}` déjà utilisée",
                method, register_user.email
            );
            return (StatusCode::FORBIDDEN, "Adresse email déjà utilisée").into_response();
        }
        None => (),
    };

    let refresh_token = match create_jwt(None, app_state.secret_key.as_bytes(), Duration::days(365))
    {
        Ok(token) => token,
        Err(e) => {
            warn!("{} /register {}", method, e);
            return e.into_response();
        }
    };

    let email_confirm_token = match create_jwt(
        Some(register_user.email.to_string()),
        app_state.secret_key.as_bytes(),
        Duration::minutes(5),
    ) {
        Ok(jwt) => jwt,
        Err(code) => return code.into_response(),
    };

    match sqlx::query!("INSERT INTO users (username, email, password, birthdate, biography, is_male, token) VALUES ($1, $2, $3, $4, $5, $6, $7);", register_user.username, email_confirm_token, password, birthdate, register_user.biography, register_user.is_male, refresh_token).execute(&app_state.pool).await {
        Ok(_) => (),
        Err(e) => {
            warn!("{} /register {}", method, e);
            return e.to_string().into_response();
        }
    };

    match send_html_message(
        app_state.smtp_client,
        "Confirm email",
        &format!("<h1>Un compte a été créé en utilisant cette adresse email, si vous êtes à l’origine de cette action, cliquez <a href='{}/register/email_confirm?token={}'>ici</a> pour l'activer, sinon vous pouvez ignorer cet email.</h1>", API_URL, email_confirm_token),
        register_user.email.parse().unwrap(),
    ) {
        Ok(_) => (),
        Err(e) => {
            warn!("{} /register {}", method, e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    StatusCode::OK.into_response()
}
