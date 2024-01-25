use std::sync::Arc;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    headers::{authorization::Bearer, Authorization},
    http::request::Parts,
    TypedHeader,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::{
    utils::{app_error::AppError, authentification::authentificate},
    AppState,
};

#[derive(Serialize, Deserialize)]
pub struct InnerAuthUser {
    pub id: i64,
}

pub struct AuthUser(pub Option<InnerAuthUser>);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);
        let typed_header =
            match TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await {
                Ok(header) => header,
                Err(e) => {
                    info!("User not connected : {e}");
                    return Ok(AuthUser(None));
                }
            };
        authentificate(app_state, typed_header.token()).await
    }
}
