use crate::structs::register_user::RegisterUser;
use crate::utils::app_error::AppError;
use email_address::EmailAddress;
use hyper::StatusCode;
use lettre::{
    message::{header::ContentType, Mailbox},
    Address, Message, SmtpTransport, Transport,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfos {
    pub id: i64,
    pub permission: usize,
}

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub id: i64,
}

pub fn send_html_message(
    smtp_client: &SmtpTransport,
    subject: &str,
    msg: &str,
    to: Address,
) -> Result<(), AppError> {
    smtp_client
        .send(
            &Message::builder()
                .from(Mailbox {
                    name: Some(env!("EMAIL_NAME").to_string()),
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
        )
        .map_err(|e| {
            warn!("Error while sending email : {e}");
            AppError::internal_server_error()
        })?;
    Ok(())
}

pub fn check_username(username: &str) -> Result<(), AppError> {
    if username.len() < 5 || username.len() > 12 {
        warn!("Wrong username size : {username}");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Le nom d'utilisateur doit contenir entre 5 et 12 caractères."),
        ));
    }

    for (i, c) in username.char_indices() {
        if i == 0 {
            if !c.is_alphabetic() {
                warn!("The username has to begin with a letter : {username}");
                return Err(AppError::new(
                    StatusCode::FORBIDDEN,
                    Some("Le nom d'utilisateur doit commencer par une lettre."),
                ));
            }
            continue;
        }
        if !c.is_alphanumeric() && c != '_' {
            warn!("The username has to contain only letters, digits and underscores : {username}");
            return Err(AppError::new(StatusCode::FORBIDDEN, Some("Le nom d'utilisateur ne doit contenir que des lettres, des chiffres et des underscores.")));
        }
    }

    Ok(())
}

pub fn check_email_address(email: &str) -> Result<(), AppError> {
    if !EmailAddress::is_valid(email) {
        warn!("Invalid email `{email}`");
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("L'email est invalide."),
        ));
    }
    Ok(())
}

pub fn check_register_infos(user: &RegisterUser) -> Result<(), AppError> {
    check_username(&user.username)?;

    check_email_address(&user.email)?;

    if user.password.len() < 8 {
        warn!("Password `{}` too short", user.password);
        return Err(AppError::new(
            StatusCode::FORBIDDEN,
            Some("Mot de passe trop court."),
        ));
    }

    Ok(())
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha512::new();
    hasher.update(password);
    format!("{:x}", hasher.finalize())
}
