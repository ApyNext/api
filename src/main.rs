mod middlewares;
mod routes;
mod structs;
mod utils;

use std::sync::Arc;

use axum::Router;
use axum::{middleware, routing::post};
use libaes::Cipher;
use shuttle_runtime::tracing::warn;
use shuttle_runtime::Service;

use crate::utils::delete_not_activated_expired_accounts::delete_not_activated_expired_accounts;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use middlewares::logger_middleware::logger_middleware;
use routes::email_confirm_route::email_confirm_route;
use routes::register_route::register_route;
use shuttle_secrets::SecretStore;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pool: PgPool,
    smtp_client: SmtpTransport,
    cipher: Arc<Cipher>,
}

pub struct CustomService {
    router: Router,
    pool: PgPool,
}

//TODO change by front URL
const API_URL: &str = "https://apynext.shuttleapp.rs";

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_secrets::Secrets] secrets: SecretStore,
    #[shuttle_shared_db::Postgres(local_uri = "{secrets.DATABASE_URL}")] pool: PgPool,
) -> Result<CustomService, shuttle_runtime::Error> {
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let smtp_client = SmtpTransport::relay(&secrets.get("EMAIL_SMTP_SERVER").unwrap())
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

    if secret_key.len() != 32 {
        panic!("La clé d'encryption doit avoir une taille de 32 bytes");
    }

    let app_state = AppState {
        pool: pool.clone(),
        smtp_client,
        //Change to safe key
        cipher: Arc::new(Cipher::new_256(&secret_key.as_bytes().try_into().unwrap())),
    };

    let router = Router::new()
        .route("/register", post(register_route))
        .route("/register/email_confirm", post(email_confirm_route))
        .layer(middleware::from_fn(logger_middleware))
        .with_state(app_state);

    Ok(CustomService { pool, router })
}

#[shuttle_runtime::async_trait]
impl Service for CustomService {
    async fn bind(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        let serve_router = axum::Server::bind(&addr).serve(self.router.into_make_service());

        tokio::select! {
            _ = delete_not_activated_expired_accounts(&self.pool) => {
                warn!("This should never happen");
            },
            _ = serve_router => {}
        }

        Ok(())
    }
}
