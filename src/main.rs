mod middleware;
mod routes;
mod structs;
mod utils;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use axum::extract::ws::{Message, WebSocket};
use axum::{
    middleware as axum_middleware,
    routing::{get, post},
};
use axum::{Extension, Router};
use libaes::Cipher;
use routes::ws::ws_route;
use shuttle_runtime::tracing::{info, warn};
use shuttle_runtime::Service;
use tokio::sync::mpsc::UnboundedSender;

use crate::utils::delete_not_activated_expired_accounts::delete_not_activated_expired_accounts;
use hyper::header::HeaderValue;
use hyper::http::Method;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use middleware::logger_middleware::logger_middleware;
use routes::a2f_login_route::a2f_login_route;
use routes::email_confirm_route::email_confirm_route;
use routes::login_route::login_route;
use routes::register_route::register_route;
use shuttle_secrets::SecretStore;
use sqlx::PgPool;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::CorsLayer;

type Users = Arc<RwLock<HashMap<usize, UnboundedSender<Message>>>>;
static NEXT_USER_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

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

const FRONT_URL: &str = "https://apynext.creativeblogger.org";

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

    if smtp_client.test_connection().unwrap() {
        info!("Connexion SMTP effectuée avec succès !");
    }

    let secret_key = secrets
        .get("ENCODING_KEY")
        .expect("Please set ENCODING_KEY value in Secrets.toml");

    if secret_key.len() != 32 {
        panic!("La clé d'encryption doit avoir une taille de 32 bytes");
    }

    let app_state = AppState {
        pool: pool.clone(),
        smtp_client,
        cipher: Arc::new(Cipher::new_256(&secret_key.as_bytes().try_into().unwrap())),
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_origin(FRONT_URL.parse::<HeaderValue>().unwrap());

    let router = Router::new()
        .route("/register", post(register_route))
        .route("/register/email_confirm", post(email_confirm_route))
        .route("/login", post(login_route))
        .route("/login/a2f", post(a2f_login_route))
        .route("/ws", get(ws_route))
        .layer(cors)
        .layer(axum_middleware::from_fn(logger_middleware))
        .layer(CookieManagerLayer::new())
        .layer(Extension(Users::default()))
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
