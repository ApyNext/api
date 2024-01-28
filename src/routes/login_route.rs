use std::sync::Arc;

use crate::utils::app_error::AppError;
use crate::utils::register::hash_password;
use crate::utils::register::send_html_message;
use crate::utils::register::{check_email_address, check_username};
use crate::utils::token::Token;
use crate::AppState;
use crate::FRONT_URL;
use axum::extract::State;
use axum::Json;
use chrono::Duration;
use hyper::StatusCode;
use lettre::Address;
use serde::Deserialize;
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

#[derive(Deserialize)]
pub struct LoginUser {
    pub username_or_email: String,
    pub password: String,
}

pub async fn login_route(
    State(app_state): State<Arc<AppState>>,
    Json(register_user): Json<LoginUser>,
) -> Result<StatusCode, AppError> {
    let username_or_email = register_user.username_or_email.to_lowercase();
    let password = hash_password(&register_user.password);
    drop(register_user);
    let user = if username_or_email.contains('@') {
        check_email_address(&username_or_email)?;
        let user = sqlx::query_as!(
            UserForLoginA2FWithoutEmail,
            "SELECT username, token FROM account WHERE email = $1 AND password = $2",
            username_or_email,
            password
        )
        .fetch_one(&app_state.pool)
        .await
        .map_err(|e| {
            warn!("Error getting user with email `{username_or_email}` from database : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Identifiants invalides."))
        })?;
        UserForLoginA2F {
            username: user.username,
            email: username_or_email,
            token: user.token,
        }
    } else {
        check_username(&username_or_email)?;
        let user = sqlx::query_as!(
            UserForLoginA2FWithoutUsername,
            "SELECT email, token FROM account WHERE username = $1 AND password = $2",
            username_or_email,
            password
        )
        .fetch_one(&app_state.pool)
        .await
        .map_err(|e| {
            warn!("Error getting user @{username_or_email} from database : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Identifiants invalides."))
        })?;
        UserForLoginA2F {
            username: username_or_email,
            email: user.email,
            token: user.token,
        }
    };

    let a2f_token = Token::create(user.token, Duration::minutes(10), &app_state.cipher);

    let a2f_token = urlencoding::encode(&a2f_token).to_string();

    let email = user.email.parse::<Address>().map_err(|e| {
        warn!("Cannot parse email `{}` : {}", user.email, e);
        AppError::new(StatusCode::FORBIDDEN, Some("Email invalide."))
    })?;

    send_html_message(
        &app_state.smtp_client,
        "Vérifier la connexion",
        &format!("<p>Heureux de te revoir <b>@{}</b> !\nQuelqu'un a tenté de se connecter à ton compte, si tu es à l’origine de cette action, clique <a href='{FRONT_URL}{}?token={a2f_token}'>ici</a> pour te connecter.\nTu peux également copier-coller directement le token de connexion directement :<div><code>{a2f_token}<code><div>\nSi tu n'es pas à l'origine de cette action, tu peux ignorer cet email, nous te conseillons néanmoins de changer ton mot de passe.</p>", user.username, env!("A2F_ROUTE")),
        email,
    )?;

    Ok(StatusCode::OK)
}
