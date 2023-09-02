use crate::structs::register_user::RegisterUser;
use crate::utils::app_error::AppError;
use email_address::EmailAddress;
use lettre::{
    message::{header::ContentType, Mailbox},
    Address, Message, SmtpTransport, Transport,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use shuttle_runtime::tracing::warn;

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
        }
    }
}

pub fn check_username(username: &str) -> Result<(), AppError> {
    if username.len() < 5 || username.len() > 12 {
        return Err(AppError::IncorrectUsernameLength);
    }

    for (i, c) in username.char_indices() {
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

    Ok(())
}

pub fn check_email_address(email: &str) -> Result<(), AppError> {
    if !EmailAddress::is_valid(email) {
        return Err(AppError::InvalidEmail);
    }
    Ok(())
}

pub fn check_register_infos(user: &RegisterUser) -> Result<(), AppError> {
    check_username(&user.username)?;

    check_email_address(&user.email)?;

    if user.password.len() < 8 {
        return Err(AppError::PasswordTooShort);
    }

    Ok(())
}

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha512::new();
    hasher.update(password);
    format!("{:x}", hasher.finalize())
}
