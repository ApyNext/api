use crate::utils::app_error::AppError;
use base64::{engine::general_purpose, Engine};
use chrono::{Duration, Utc};
use hyper::StatusCode;
use libaes::Cipher;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

/// Struct that represents a serialized token
#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    exp: i64,
    sub: String,
}

impl Token {
    /// Create an encrypted and encoded token
    pub fn create(sub: String, exp_in: Duration, cipher: &Cipher) -> String {
        // Get expiration timestamp
        let exp = (Utc::now() + exp_in).timestamp();

        // Get serialized Claims
        let claims = json!(Token { exp, sub }).to_string();

        // Generate nonce
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut nonce);

        // Create plaintext
        let plaintext = claims.as_bytes();

        //Encrypt data
        let encrypted = cipher.cbc_encrypt(&nonce, plaintext);
        //Encode data with nonce at the beggining
        general_purpose::STANDARD.encode([&nonce, encrypted.as_slice()].concat())
    }

    /// Decode token and return its content or an error
    pub fn decode(token: &str, cipher: &Cipher) -> Result<String, AppError> {
        //Decode the token
        let encyrpted_decoded = general_purpose::STANDARD.decode(token).map_err(|e| {
            warn!("Error decoding token : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Token invalide."))
        })?;
        //Split the nonce and the data
        let nonce = &encyrpted_decoded[..16];
        let datas = &encyrpted_decoded[16..];
        //Decrypt the token
        let decrypted = cipher.cbc_decrypt(nonce, datas);
        //Convert it to String
        let string_decrypted = String::from_utf8(decrypted).map_err(|e| {
            warn!("Error decrypting token : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Token invalide."))
        })?;

        //Deserialize token
        let token: Token = serde_json::from_str(&string_decrypted).map_err(|e| {
            warn!("Error deserializing token `{string_decrypted}` : {e}");
            AppError::new(StatusCode::FORBIDDEN, Some("Token invalide."))
        })?;
        //Check if the token is expired
        if token.exp <= Utc::now().timestamp() {
            warn!(
                "Expired token {}, expire timestamp : {}",
                token.sub, token.exp
            );
            return Err(AppError::new(StatusCode::FORBIDDEN, Some("Token expiré.")));
        }

        // Return the content of the token
        Ok(token.sub)
    }
}
