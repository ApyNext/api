use axum::response::{IntoResponse, Response};
use base64::{engine::general_purpose, Engine};
use chrono::{Duration, Utc};
use hyper::StatusCode;
use libaes::Cipher;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shuttle_runtime::tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    sub: String,
}

pub fn create_token(sub: String, key: &[u8], exp_in: Duration) -> Result<String, String> {
    //Get expiration timestamp
    let exp = (Utc::now() + exp_in).timestamp() as usize;

    //Get serialized Claims
    let claims = json!(Claims { exp, sub }).to_string();

    println!("{}", claims);

    //Create cipher
    let cipher = Cipher::new_256(b"12345678901234567890123456789012");

    //Generate nonce
    let mut nonce = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce);

    //Create plaintext
    let plaintext = claims.as_bytes();

    //Encrypt data
    let encrypted = cipher.cbc_encrypt(&nonce, plaintext);
    println!("{:?}", encrypted);
    //Encode data with nonce at the beggining
    let encrypted_encoded =
        general_purpose::STANDARD.encode([&nonce, encrypted.as_slice()].concat());
    println!("{}", encrypted_encoded);
    Ok(encrypted_encoded)
}

pub fn decode_email_token(jwt: &str, key: &[u8]) -> Result<String, Response> {
    let cipher = Cipher::new_256(b"12345678901234567890123456789012");

    //Decode datas
    let encyrpted_decoded = general_purpose::STANDARD.decode(jwt).unwrap();
    //Decrypt datas
    let nonce = &encyrpted_decoded[..16];
    let datas = &encyrpted_decoded[16..];
    let decrypted = cipher.cbc_decrypt(nonce, datas);
    let string_decrypted = String::from_utf8(decrypted).unwrap();
    info!("{}", string_decrypted);

    let claims: Claims = serde_json::from_str(&string_decrypted).unwrap();

    if claims.exp <= Utc::now().timestamp() as usize {
        return Err((StatusCode::FORBIDDEN, "Lien de validation d'email expiré").into_response());
    }

    Ok(claims.sub)
}

pub fn decode_refresh_jwt(jwt: &str, secret: &[u8]) -> Result<String, Response> {
    let cipher = Cipher::new_256(b"12345678901234567890123456789012");

    //Decode datas
    let encyrpted_decoded = general_purpose::STANDARD.decode(jwt).unwrap();
    //Decrypt datas
    let nonce = &encyrpted_decoded[..16];
    let datas = &encyrpted_decoded[16..];
    let decrypted = cipher.cbc_decrypt(nonce, datas);
    let string_decrypted = String::from_utf8(decrypted).unwrap();
    info!("{}", string_decrypted);

    let claims: Claims = serde_json::from_str(&string_decrypted).unwrap();

    if claims.exp <= Utc::now().timestamp() as usize {
        return Err((StatusCode::FORBIDDEN, "Lien de validation de token expiré").into_response());
    }

    Ok(claims.sub)
}
