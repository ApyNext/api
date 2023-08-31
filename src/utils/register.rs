use email_address::EmailAddress;
use lettre::{
    message::{header::ContentType, Mailbox},
    Address, Message, SmtpTransport, Transport,
};
use serde::{Deserialize, Serialize};
use shuttle_runtime::tracing::warn;

use crate::structs::register_user::RegisterUser;
use crate::utils::app_error::AppError;

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
    smtp_client: SmtpTransport,
    subject: &str,
    msg: &str,
    to: Address,
    header: &str,
) -> Result<(), AppError> {
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
        Err(e) => {
            warn!("{} Error while sending email : {}", header, e);
            Err(AppError::EmailSendError)
        },
    }
}

pub fn check_register_infos(user: &RegisterUser) -> Result<(), AppError> {
    if user.username.len() < 5 || user.username.len() > 12 {
        return Err(AppError::IncorrectUsernameLength);
    }

    for (i, c) in user.username.char_indices() {
        if i == 0 {
            if !c.is_alphabetic() {
                return Err(AppError::UsernameMustBeginByALetter);
            }
            continue;
        }
        if !c.is_alphanumeric() && c != '_' {
            return Err(AppError::UsernameMustOnlyContainLettersDigitsAndUnderscores);
        }
    }

    if !EmailAddress::is_valid(&user.email) {
        return Err(AppError::InvalidEmail);
    }

    if user.password.len() < 8 {
        return Err(AppError::PasswordTooShort);
    }

    if user.biography.len() >= 300 {
        return Err(AppError::BiographyTooLong);
    }

    Ok(())
}
