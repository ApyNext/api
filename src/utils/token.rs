use crate::utils::app_error::AppError;
use base64::{engine::general_purpose, Engine};
use chrono::{Duration, Utc};
use hyper::StatusCode;
use libaes::Cipher;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    exp: i64,
    sub: String,
}

impl Token {
    pub fn new(sub: String, exp_in: Duration, cipher: &Cipher) -> String {
        //Get expiration timestamp
        let exp = (Utc::now() + exp_in).timestamp();

        //Get serialized Claims
        let claims = json!(Token { exp, sub }).to_string();

        //Generate nonce
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut nonce);

        //Create plaintext
        let plaintext = claims.as_bytes();

        //Encrypt data
        let encrypted = cipher.cbc_encrypt(&nonce, plaintext);
        //Encode data with nonce at the beggining
        general_purpose::STANDARD.encode([&nonce, encrypted.as_slice()].concat())
    }

    pub fn decode(token: &str, cipher: &Cipher) -> Result<String, AppError> {
        //Decode datas
        let encyrpted_decoded = general_purpose::STANDARD.decode(token).map_err(|e| {
            warn!("Error decoding token : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Token invalide."))
        })?;
        //Decrypt datas
        let nonce = &encyrpted_decoded[..16];
        let datas = &encyrpted_decoded[16..];
        let decrypted = cipher.cbc_decrypt(nonce, datas);
        let string_decrypted = String::from_utf8(decrypted).map_err(|e| {
            warn!("Error decrypting token : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Token invalide."))
        })?;

        //Get claims
        let token: Token = serde_json::from_str(&string_decrypted).map_err(|e| {
            warn!("Error deserializing token `{string_decrypted}` : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Token invalide."))
        })?;
        //Check if the token is expired
        if token.exp <= Utc::now().timestamp() {
            warn!("Expired token");
            return Err(AppError::new(StatusCode::FORBIDDEN, Some("Token expiré.")));
        }

        Ok(token.sub)
    }
}
