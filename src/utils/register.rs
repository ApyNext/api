use email_address::EmailAddress;
use lettre::{
    message::{header::ContentType, Mailbox},
    Address, Message, SmtpTransport, Transport,
};
use serde::{Deserialize, Serialize};
use axum::response::{Response, IntoResponse};
use hyper::StatusCode;

use crate::structs::register_user::RegisterUser;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfos {
    pub id: i64,
    pub permission: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub id: i64,
}

pub enum AppError {
    //Token decoding errors
    InvalidToken,
    ExpiredToken
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = match self {
            AppError::InvalidToken => "Token invalide",
            AppError::ExpiredToken => "Token expiré"
        };

        let status_code = match self {
            AppError::InvalidToken | AppError::ExpiredToken => StatusCode::FORBIDDEN,
            //Add errors here
        };

        (status_code, body).into_response()
    }
}

pub fn send_html_message(
    smtp_client: SmtpTransport,
    subject: &str,
    msg: &str,
    to: Address,
) -> Result<(), String> {
    match smtp_client.send(
        &Message::builder()
            .from(Mailbox {
                name: Some("ApyNext".to_string()),
                email: Address::new("email.confirmation", "creativeblogger.org").unwrap(),
            })
            .to(Mailbox {
                name: None,
                email: to,
            })
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(msg.to_string())
            .unwrap(),
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn check_register_infos(user: &RegisterUser) -> Result<(), String> {
    if user.username.len() < 5 || user.username.len() > 12 {
        return Err(
            "Le nom d'utilisateur doit contenir entre 5 et 12 caractères compris.".to_string(),
        );
    }

    for (i, c) in user.username.char_indices() {
        if i == 0 {
            if !c.is_alphabetic() {
                return Err("Le nom d'utilisateur doit commencer par une lettre.".to_string());
            }
            continue;
        }
        if !c.is_alphanumeric() && c != '_' {
            return Err("Le nom d'utilisateur ne peut contenir que des lettres, des chiffres et des underscores.".to_string());
        }
    }

    if !EmailAddress::is_valid(&user.email) {
        return Err("L'adresse email n'est pas valide.".to_string());
    }

    if user.password.len() < 8 {
        return Err("Le mot de passe doit faire au moins 8 caractères.".to_string());
    }

    if user.biography.len() >= 300 {
        return Err("La biographie doit contenir au maximum 300 caractères.".to_string());
    }

    Ok(())
}
