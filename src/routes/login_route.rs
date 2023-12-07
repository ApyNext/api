use std::sync::Arc;

use crate::structs::login_user::LoginUser;
use crate::utils::app_error::AppError;
use crate::utils::register::hash_password;
use crate::utils::register::send_html_message;
use crate::utils::register::{check_email_address, check_username};
use crate::utils::token::create_token;
use crate::AppState;
use crate::FRONT_URL;
use axum::extract::State;
use axum::Json;
use chrono::Duration;
use hyper::Method;
use hyper::StatusCode;
use lettre::Address;
use tracing::warn;

struct UserForLoginA2F {
    username: String,
    email: String,
    token: String,
}

struct UserForLoginA2FWithoutUsername {
    email: String,
    token: String,
}

struct UserForLoginA2FWithoutEmail {
    username: String,
    token: String,
}

pub async fn login_route(
    method: Method,
    State(app_state): State<Arc<AppState>>,
    Json(register_user): Json<LoginUser>,
) -> Result<StatusCode, AppError> {
    let username_or_email = register_user.username_or_email.to_lowercase();
    let password = hash_password(&register_user.password);
    drop(register_user);
    let user = if username_or_email.contains('@') {
        check_email_address(&username_or_email)?;
        let user = match sqlx::query_as!(
            UserForLoginA2FWithoutEmail,
            "SELECT username, token FROM users WHERE email = $1 AND password = $2",
            username_or_email,
            password
        )
        .fetch_one(&app_state.pool)
        .await
        {
            Ok(user) => user,
            Err(e) => {
                warn!("{} /login Error while login : {}", method, e);
                return Err(AppError::IncorrectCredentials);
            }
        };
        UserForLoginA2F {
            username: user.username,
            email: username_or_email,
            token: user.token,
        }
    } else {
        check_username(&username_or_email)?;
        let user = match sqlx::query_as!(
            UserForLoginA2FWithoutUsername,
            "SELECT email, token FROM users WHERE username = $1 AND password = $2",
            username_or_email,
            password
        )
        .fetch_one(&app_state.pool)
        .await
        {
            Ok(auth_token) => auth_token,
            Err(e) => {
                warn!("{} /login Error while login : {}", method, e);
                return Err(AppError::IncorrectCredentials);
            }
        };
        UserForLoginA2F {
            username: username_or_email,
            email: user.email,
            token: user.token,
        }
    };

    let a2f_token = create_token(user.token, Duration::minutes(10), &app_state.cipher);

    let a2f_token = urlencoding::encode(&a2f_token).to_string();

    let email = match user.email.parse::<Address>() {
        Ok(email) => email,
        Err(e) => {
            warn!("{} /login Cannot parse email : {}", method, e);
            return Err(AppError::InvalidEmail);
        }
    };

    send_html_message(
        &app_state.smtp_client,
        "Vérifier la connexion",
        &format!("<p>Heureux de te revoir <b>@{}</b> ! Quelqu'un a tenté de se connecter à votre compte, si vous êtes à l’origine de cette action, cliquez <a href='{}/login/a2f?token={}'>ici</a> pour vous connecter, sinon vous pouvez ignorer cet email.</p>", user.username, FRONT_URL, a2f_token),
        email,
        &format!("{method} /login"),
    )?;

    Ok(StatusCode::OK)
}
