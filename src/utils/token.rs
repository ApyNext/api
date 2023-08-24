use aes_gcm::{
    aead::{
        generic_array::{sequence::Lengthen, GenericArray},
        Aead, OsRng,
    },
    AeadCore, Aes256Gcm, Key, KeyInit,
};
use axum::response::{IntoResponse, Response};
use base64::{engine::general_purpose, Engine};
use chrono::{Duration, Utc};
use hyper::StatusCode;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::digest::typenum::UInt;
use shuttle_runtime::tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    sub: String,
}

pub fn create_token(sub: String, key: &[u8], exp_in: Duration) -> Result<String, String> {
    let exp = (Utc::now() + exp_in).timestamp() as usize;
    let claims = json!(Claims { exp, sub });
    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(&key);
    let mut nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let plaintext = nonce.to_vec();
    plaintext.extend_from_slice(b"slt".as_slice());
    let encrypted = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();
    info!("{:?}", encrypted);
    let encrypted_encoded = general_purpose::STANDARD.encode(&encrypted);
    info!("{}", encrypted_encoded);
    let decrypted = cipher.decrypt(&nonce, encrypted.as_ref()).unwrap();
    info!("{:?}", decrypted);
    Ok(encrypted_encoded)
}

pub fn decode_email_token(jwt: &str, key: &[u8]) -> Result<String, Response> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(&key);

    let decoded_encrypted = general_purpose::STANDARD.decode(jwt).unwrap();
    let nonce = GenericArray::from_slice(&decoded_encrypted[..12]);

    Ok("".to_string())
}

pub fn decode_refresh_jwt(jwt: &str, secret: &[u8]) -> Result<(), Response> {
    match decode::<Claims>(
        jwt,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(_) => Ok(()),
        Err(e) => match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                Err((StatusCode::FORBIDDEN, "Lien de validation expiré.").into_response())
            }
            _ => Err((StatusCode::UNAUTHORIZED, "Lien invalide.").into_response()),
        },
    }
}
