use chrono::prelude::*;
use chrono::Duration;
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
use shuttle_secrets::SecretStore;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iat: usize,
    sub: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfos {
    id: i128,
    permission: usize,
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

pub fn create_jwt(user_infos: UserInfos, secrets: SecretStore) -> Result<String, StatusCode> {
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
        &EncodingKey::from_secret(
            secrets
                .get("ENCODING_KEY")
                .expect("Veuillez renseigner le secret `ENCODING_KEY` dans Secrets.toml")
                .as_bytes(),
        ),
    )
    .map_err(|e| {
        error!("Erreur lors de la création du token");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
