use lettre::{
    message::{header::ContentType, Mailbox},
    Address, Message, SmtpTransport, Transport,
};
use rand::distributions::{Alphanumeric, DistString};

pub fn generate_token() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 256)
}

pub fn send_html_message(smtp_client: SmtpTransport, msg: &str, to: Address) -> Result<(), String> {
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
            .subject("slt")
            .header(ContentType::TEXT_HTML)
            .body(msg.to_string())
            .unwrap(),
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
