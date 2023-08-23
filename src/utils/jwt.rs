use axum::response::{IntoResponse, Response};
use chrono::{Duration, Utc};
use hyper::StatusCode;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    iat: usize,
    sub: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshClaims {
    exp: usize,
    iat: usize,
}

pub fn create_jwt(sub: Option<String>, secret: &[u8], exp_in: Duration) -> Result<String, String> {
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + exp_in).timestamp() as usize;
    let claims = match sub {
        Some(sub) => json!(Claims { iat, exp, sub }),
        None => json!(RefreshClaims { iat, exp }),
    };
    match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    ) {
        Ok(jwt) => Ok(jwt),
        Err(e) => Err(format!(
            "Erreur lors de la génération du JWT (avec une expiration : {}) : {}",
            exp_in, e
        )),
    }
}

pub fn decode_email_jwt(jwt: &str, secret: &[u8]) -> Result<String, Response> {
    match decode::<Claims>(
        jwt,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(token) => Ok(token.claims.sub),
        Err(e) => match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                Err((StatusCode::FORBIDDEN, "Lien de validation expiré.").into_response())
            }
            _ => Err((StatusCode::UNAUTHORIZED, "Lien invalide.").into_response()),
        },
    }
}

pub fn decode_refresh_jwt(jwt: &str, secret: &[u8]) -> Result<(), Response> {
    match decode::<RefreshClaims>(
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
