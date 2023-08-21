use chrono::prelude::*;
use chrono::Duration;
use email_address::EmailAddress;
use hyper::StatusCode;
use jsonwebtoken::encode;
use jsonwebtoken::EncodingKey;
use jsonwebtoken::Header;
use lettre::{
    message::{header::ContentType, Mailbox},
    Address, Message, SmtpTransport, Transport,
};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shuttle_runtime::tracing::log::error;

use crate::structs::register_user::RegisterUser;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iat: usize,
    sub: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfos {
    pub id: i128,
    pub permission: usize,
}

pub fn generate_token() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 256)
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
                name: Some("Email confirmation".to_string()),
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

pub fn create_jwt(user_infos: UserInfos, secret: &[u8]) -> Result<String, StatusCode> {
    let mut now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp_in = Duration::minutes(15);
    now += exp_in;
    let exp = now.timestamp() as usize;
    let sub = json!(user_infos).to_string();
    let claims = Claims { iat, exp, sub };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| {
        error!("Erreur lors de la création du token : {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

pub fn create_email_jwt(email: String, secret: &[u8]) -> Result<String, StatusCode> {
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::minutes(15)).timestamp() as usize;
    let claims = Claims {
        iat,
        exp,
        sub: email,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| {
        error!("Erreur lors de la création du token d'email : {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
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
