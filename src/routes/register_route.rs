use std::sync::Arc;

use axum::{extract::State, Json};
use chrono::Duration;
use hyper::StatusCode;
use lettre::Address;
use time::OffsetDateTime;
use tracing::warn;

use crate::utils::app_error::AppError;
use crate::utils::register::check_register_infos;
use crate::utils::register::hash_password;
use crate::utils::token::Token;
use crate::FRONT_URL;
use crate::{structs::register_user::RegisterUser, utils::register::send_html_message, AppState};

pub async fn register_route(
    State(app_state): State<Arc<AppState>>,
    Json(mut register_user): Json<RegisterUser>,
) -> Result<StatusCode, AppError> {
    register_user.username = register_user.username.to_lowercase();
    register_user.email = register_user.email.to_lowercase();
    check_register_infos(&register_user)?;

    let email = register_user.email.parse::<Address>().map_err(|e| {
        warn!("Cannot parse email `{}` : {}", register_user.email, e);
        AppError::new(StatusCode::FORBIDDEN, Some("Email invalide."))
    })?;

    let password = hash_password(&register_user.password);

    let birthdate = OffsetDateTime::from_unix_timestamp(register_user.birthdate).map_err(|e| {
        warn!("Invalid birthdate `{}` : {}", register_user.birthdate, e);
        AppError::new(StatusCode::FORBIDDEN, Some("Date de naissance invalide."))
    })?;

    if birthdate.year() < 1900 || birthdate > OffsetDateTime::now_utc() {
        warn!("La date de naissance doit être située entre 1900 et maintenant.");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Date de naissance invalide."),
        ));
    }

    //Check if email is already used
    if sqlx::query!("SELECT id FROM users WHERE email = $1", register_user.email)
        .fetch_optional(&app_state.pool)
        .await
        .map_err(|e| {
            warn!(
                "Error checking if the email `{}` exists in the database : {}",
                register_user.email, e
            );
            AppError::internal_server_error()
        })?
        .is_some()
    {
        warn!(
            "Email `{}` already exists in the database",
            register_user.email
        );
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Email déjà utilisé."),
        ));
    };

    //Check if username is already used
    if sqlx::query!(
        "SELECT id FROM users WHERE username = $1",
        register_user.username
    )
    .fetch_optional(&app_state.pool)
    .await
    .map_err(|e| {
        warn!(
            "Error checking if the username `{}` already exists in the database : {}",
            register_user.username, e
        );
        AppError::internal_server_error()
    })?
    .is_some()
    {
        warn!(
            "Username `{}` already exists in the database",
            register_user.username
        );
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Nom d'utilisateur déjà utilisé."),
        ));
    };

    let email_confirm_token = Token::new(
        register_user.email.clone(),
        Duration::minutes(10),
        &app_state.cipher,
    );

    sqlx::query!("INSERT INTO users (username, email, password, birthdate, is_male, token) VALUES ($1, $2, $3, $4, $5, $6);", register_user.username, email_confirm_token, password, birthdate, register_user.is_male, email_confirm_token).execute(&app_state.pool).await.map_err(|e| {
        warn!("Error creating account for `{}` : {}", register_user.username, e);
        AppError::internal_server_error()
    })?;
    let email_confirm_token = urlencoding::encode(&email_confirm_token).to_string();

    send_html_message(
        &app_state.smtp_client,
        "Vérification d'email",
        &format!("<p>Bienvenue <b>@{}</b> ! Un compte a été créé en utilisant cette adresse email, si tu es à l’origine de cette action, clique <a href='{FRONT_URL}{}?token={email_confirm_token}'>ici</a> pour l'activer.\nTu peux également copier-coller le token directement :<div><code>{email_confirm_token}</code></div>.\nSi tu n'es pas à l'origine de cette action, tu peux ignorer cet email.</p>", register_user.username, env!("EMAIL_CONFIRM_ROUTE")),
        email,
    )?;

    Ok(StatusCode::OK)
}
