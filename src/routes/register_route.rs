use std::sync::Arc;

use axum::{extract::State, Json};
use chrono::Duration;
use hyper::Method;
use hyper::StatusCode;
use lettre::Address;
use time::OffsetDateTime;
use tracing::warn;

use crate::extractors::auth_extractor::InnerAuthUser;
use crate::utils::app_error::AppError;
use crate::utils::register::check_register_infos;
use crate::utils::register::hash_password;
use crate::utils::token::create_token;
use crate::FRONT_URL;
use crate::{structs::register_user::RegisterUser, utils::register::send_html_message, AppState};

pub async fn register_route(
    method: Method,
    State(app_state): State<Arc<AppState>>,
    Json(mut register_user): Json<RegisterUser>,
) -> Result<StatusCode, AppError> {
    register_user.username = register_user.username.to_lowercase();
    register_user.email = register_user.email.to_lowercase();
    check_register_infos(&register_user)?;

    let email = match register_user.email.parse::<Address>() {
        Ok(email) => email,
        Err(e) => {
            warn!("{} /register Cannot parse email : {}", method, e);
            return Err(AppError::InvalidEmail);
        }
    };

    let password = hash_password(&register_user.password);

    let birthdate = match OffsetDateTime::from_unix_timestamp(register_user.birthdate) {
        Ok(birthdate) => birthdate,
        Err(e) => {
            warn!("{} /register Invalid birthdate : {}", method, e);
            return Err(AppError::InvalidBirthdate);
        }
    };

    if birthdate.year() < 1900 || birthdate > OffsetDateTime::now_utc() {
        warn!(
            "{} /register La date de naissance doit être située entre 1900 et maintenant.",
            method
        );
        return Err(AppError::InvalidBirthdate);
    }

    //Check if email is already used
    match match sqlx::query!("SELECT id FROM users WHERE email = $1", register_user.email)
        .fetch_optional(&app_state.pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!(
                "{} /register Error while checking if email address already exists : {}",
                method, e
            );
            return Err(AppError::InternalServerError);
        }
    } {
        Some(_) => {
            warn!(
                "{} /register Email address `{}` already used",
                method, register_user.email
            );
            return Err(AppError::EmailAddressAlreadyUsed);
        }
        None => (),
    };

    //Check if username is already used
    match match sqlx::query!(
        "SELECT id FROM users WHERE username = $1",
        register_user.username
    )
    .fetch_optional(&app_state.pool)
    .await
    {
        Ok(result) => result,
        Err(e) => {
            warn!(
                "{} /register Error while checking if username already exists : {}",
                method, e
            );
            return Err(AppError::InternalServerError);
        }
    } {
        Some(_) => {
            warn!(
                "{} /register Username `{}` already used",
                method, register_user.username
            );
            return Err(AppError::UsernameAlreadyUsed);
        }
        None => (),
    };

    let email_confirm_token = create_token(
        register_user.email.clone(),
        Duration::minutes(10),
        &app_state.cipher,
    );

    match sqlx::query!("INSERT INTO users (username, email, password, birthdate, is_male, token) VALUES ($1, $2, $3, $4, $5, $6);", register_user.username, email_confirm_token, password, birthdate, register_user.is_male, email_confirm_token).execute(&app_state.pool).await {
        Ok(_) => (),
        Err(e) => {
            warn!("{} /register {}", method, e);
            return Err(AppError::InternalServerError);
        }
    };

    let email_confirm_token = urlencoding::encode(&email_confirm_token).to_string();

    send_html_message(
        &app_state.smtp_client,
        "Vérification d'email",
        &format!("<p>Bienvenue <b>@{}</b> ! Un compte a été créé en utilisant cette adresse email, si vous êtes à l’origine de cette action, cliquez <a href='{}/register/email_confirm?token={}'>ici</a> pour l'activer si vous utilisez la version web ou copier coller ce code <div><code>{}</code></div> dans l'application mobile ou de bureau, sinon vous pouvez ignorer cet email.</p>", register_user.username, FRONT_URL, email_confirm_token, email_confirm_token),
        email,
        &format!("{} /register", method),
    )?;

    Ok(StatusCode::OK)
}
