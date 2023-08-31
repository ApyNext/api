use axum::extract::{Query, State};
use hyper::Method;
use serde::Deserialize;
use shuttle_runtime::tracing::warn;

use crate::{
    utils::{app_error::AppError, token::decode_token},
    AppState,
};

#[derive(Deserialize, Default)]
pub struct Token {
    pub token: String,
}

pub async fn a2f_login_route(
    method: Method,
    query: Option<Query<Token>>,
    State(app_state): State<AppState>,
) -> Result<String, AppError> {
    let Query(a2f_token) = query.unwrap_or_default();
    let a2f_token = a2f_token.token;
    if a2f_token.is_empty() {
        warn!("{} /login/a2f Token missing", method);
        return Err(AppError::TokenMissing);
    }
    let a2f_token = match urlencoding::decode(&a2f_token) {
        Ok(token) => token,
        Err(e) => {
            warn!("{} /login/a2f Error while decoding token : {}", method, e);
            return Err(AppError::InvalidToken);
        }
    }
    .to_string();

    let token = decode_token(
        &a2f_token,
        &app_state.cipher,
        &format!("{} /login/a2f", method),
    )?;

    Ok(token)
}
