use base64::{engine::general_purpose, Engine};
use chrono::{Duration, Utc};
use libaes::Cipher;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::utils::register::DecodeTokenErrorKind;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    sub: String,
}

pub fn create_token(sub: String, exp_in: Duration, cipher: &Cipher) -> Result<String, String> {
    //Get expiration timestamp
    let exp = (Utc::now() + exp_in).timestamp() as usize;

    //Get serialized Claims
    let claims = json!(Claims { exp, sub }).to_string();

    //Generate nonce
    let mut nonce = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce);

    //Create plaintext
    let plaintext = claims.as_bytes();

    //Encrypt data
    let encrypted = cipher.cbc_encrypt(&nonce, plaintext);
    //Encode data with nonce at the beggining
    let encrypted_encoded =
        general_purpose::STANDARD.encode([&nonce, encrypted.as_slice()].concat());
    Ok(encrypted_encoded)
}

pub fn decode_token(jwt: &str, cipher: &Cipher) -> Result<String, DecodeTokenErrorKind> {
    //Decode datas
    let encyrpted_decoded = match general_purpose::STANDARD.decode(jwt) {
        Ok(result) => result,
        Err(e) => return Err(DecodeTokenErrorKind::InvalidToken(format!("Error while decoding token : {}", e))),
    };
    //Decrypt datas
    let nonce = &encyrpted_decoded[..16];
    let datas = &encyrpted_decoded[16..];
    let decrypted = cipher.cbc_decrypt(nonce, datas);
    let string_decrypted = match String::from_utf8(decrypted) {
        Ok(result) => result,
        Err(e) => return Err(DecodeTokenErrorKind::InvalidToken(format!("Error while decrypting token : {}", e)))
    };

    //Get claims
    let claims: Claims = match serde_json::from_str(&string_decrypted) {
        Ok(claims) => claims,
        Err(e) => return Err(DecodeTokenErrorKind::InvalidToken(format!("Error while deserializing token to Claims : {}", e)))
    };

    //Check if the token is expired
    if claims.exp <= Utc::now().timestamp() as usize {
        return Err(DecodeTokenErrorKind::ExpiredToken);
    }

    Ok(claims.sub)
}
