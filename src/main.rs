mod middlewares;
mod routes;
mod structs;
mod utils;

use std::sync::Arc;

use axum::{middleware, routing::post, Router};
use libaes::Cipher;
use shuttle_runtime::tracing::warn;

use crate::utils::delete_not_activated_expired_accounts::delete_not_activated_expired_accounts;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use middlewares::logger_middleware::logger_middleware;
use routes::email_confirm_route::email_confirm_route;
use routes::register_route::register_route;
use shuttle_secrets::SecretStore;
use sqlx::{Acquire, PgPool};

#[derive(Clone)]
pub struct AppState {
    pool: PgPool,
    smtp_client: SmtpTransport,
    cipher: Arc<Cipher>,
}

//TODO change by front URL
const API_URL: &str = "https://apynext.shuttleapp.rs";

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_secrets::Secrets] secrets: SecretStore,
    #[shuttle_shared_db::Postgres(local_uri = "{secrets.DATABASE_URL}")] pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let smtp_client = SmtpTransport::relay("mail.creativeblogger.org")
        .unwrap()
        .credentials(Credentials::new(
            secrets.get("EMAIL").unwrap(),
            secrets.get("EMAIL_PASSWORD").unwrap(),
        ))
        .build();

    println!("{}", smtp_client.test_connection().unwrap());

    let secret_key = secrets
        .get("ENCODING_KEY")
        .expect("Please set ENCODING_KEY value in Secrets.toml");

    let app_state = AppState {
        pool: pool.clone(),
        smtp_client,
        //Change to safe key
        cipher: Arc::new(Cipher::new_256(b"12345678901234567890123456789012")),
    };

    let router = Router::new()
        .route("/register", post(register_route))
        .route("/register/email_confirm", post(email_confirm_route))
        .layer(middleware::from_fn(logger_middleware))
        .with_state(app_state);

    tokio::select! {
        _ = delete_not_activated_expired_accounts(&pool) => {
            warn!("This should never happen");
        }
    }

    Ok(router.into())
}
