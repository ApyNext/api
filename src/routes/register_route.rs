use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use hyper::Method;
use hyper::StatusCode;
use sha2::{Digest, Sha512};
use shuttle_runtime::tracing::{info, warn};
use shuttle_secrets::SecretStore;
use time::OffsetDateTime;

use crate::{
    structs::register_user::RegisterUser,
    utils::register::{generate_token, send_html_message},
    AppState,
};

pub async fn register_route(
    method: Method,
    State(app_state): State<AppState>,
    Json(register_user): Json<RegisterUser>,
    #[shuttle_secrets::Secrets] secrets: SecretStore,
) -> Response {
    let mut hasher = Sha512::new();
    hasher.update(register_user.password);
    let password = format!("{:x}", hasher.finalize());
    let token = generate_token();
    let email_confirm_token = generate_token();
    let birthdate = match OffsetDateTime::from_unix_timestamp(register_user.birthdate) {
        Ok(birthdate) => birthdate,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Date invalide").into_response(),
    };
    let username = register_user.username.to_lowercase();
    let email = format!(
        "{}|{}",
        email_confirm_token,
        register_user.email.to_lowercase()
    );

    match match sqlx::query!("SELECT * FROM users WHERE email = $1", email)
        .fetch_optional(&app_state.pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    } {
        Some(_) => return (StatusCode::FORBIDDEN, "Adresse email déjà utilisée").into_response(),
        None => (),
    };

    match sqlx::query!("INSERT INTO users (username, email, password, birthdate, biography, is_male, token) VALUES ($1, $2, $3, $4, $5, $6, $7);", username, email, password, birthdate, register_user.biography, register_user.is_male, token).execute(&app_state.pool).await {
        Ok(_) => (),
        Err(e) => {
            info!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

    match sqlx::query!(
        "INSERT INTO email_confirm (email, token) VALUES ($1, $2);",
        email,
        email_confirm_token
    )
    .execute(&app_state.pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            info!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

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

    "ok".into_response()
}
