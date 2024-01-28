#![warn(clippy::pedantic)]
mod extractors;
mod middleware;
mod models;
mod routes;
mod schema;
mod structs;
mod utils;

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
};
use axum::{Extension, Router};
use dotenvy::dotenv;
use libaes::Cipher;
use routes::follow_user_route::follow_user_route;
use routes::ws_route::ws_route;
use sqlx::PgPool;
use std::env::var;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicI64, AtomicUsize};
use std::sync::Arc;

use tracing::{info, warn};

use crate::routes::publish_post::publish_post_route;
use crate::utils::delete_not_activated_expired_accounts::delete_not_activated_expired_accounts;
use crate::utils::real_time_event_management::EventTracker;
use crate::utils::real_time_event_management::Users;
use hyper::header;
use hyper::header::HeaderValue;
use hyper::http::Method;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use middleware::logger::logger;
use routes::a2f_login_route::a2f_login_route;
use routes::email_confirm_route::email_confirm_route;
use routes::login_route::login_route;
use routes::ok_route::ok_route;
use routes::register_route::register_route;
use tower_http::cors::CorsLayer;

/// The global state of the app
pub struct AppState {
    pool: PgPool,
    smtp_client: SmtpTransport,
    cipher: Cipher,
}

const FRONT_URL: &str = env!("FRONT_URL");
static NEXT_NOT_CONNECTED_USER_ID: AtomicI64 = AtomicI64::new(-1);
static CONNECTED_USERS_COUNT: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();

    let Some(pool) = setup_pool().await else {
        return;
    };

    let Some(smtp_client) = setup_smtp_client() else {
        return;
    };

    let Some(cipher) = setup_cipher() else {
        return;
    };

    let app_state = Arc::new(AppState {
        pool: pool.clone(),
        smtp_client,
        cipher,
    });

    let front_url = match FRONT_URL.parse::<HeaderValue>() {
        Ok(url) => url,
        Err(e) => {
            warn!("FRONT_URL is an invalid URL : {e}");
            return;
        }
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_origin(front_url)
        .allow_headers(vec![
            header::ACCEPT,
            header::ACCEPT_LANGUAGE,
            header::CONTENT_TYPE,
            header::AUTHORIZATION, // Add other allowed headers here
        ])
        .allow_credentials(true);

    let router = Router::new()
        .route("/", get(ok_route))
        .route("/register", post(register_route))
        .route("/register/email_confirm", post(email_confirm_route))
        .route("/login", post(login_route))
        .route("/login/a2f", post(a2f_login_route))
        .route("/ws", get(ws_route))
        .route("/@:username/follow", post(follow_user_route))
        .route("/posts/new", post(publish_post_route))
        .layer(cors)
        .layer(axum_middleware::from_fn(logger))
        .layer(Extension(Users::default()))
        .layer(Extension(EventTracker::default()))
        .with_state(app_state);

    let serve_router = axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

    tokio::select! {
        () = delete_not_activated_expired_accounts(pool) => {
            warn!("This should never happen");
        },
        _ = serve_router => {}
    };
}

async fn setup_pool() -> Option<PgConnection> {
    let database_url = match var("DATABASE_URL") {
        Ok(url) => url,
        Err(e) => {
            warn!("Error getting DATABASE_URL env variable : {e}");
            return None;
        }
    };

    let pool = match PgConnection::establish(&database_url) {
        Ok(pool) => pool,
        Err(e) => {
            warn!("Error connecting to the DB : {e}");
            return None;
        }
    };

    Some(pool)
}

fn setup_smtp_client() -> Option<SmtpTransport> {
    let email_smtp_server = match var("EMAIL_SMTP_SERVER") {
        Ok(server_addr) => server_addr,
        Err(e) => {
            warn!("Error getting EMAIL_SMTP_SERVER env variable : {e}");
            return None;
        }
    };

    let email = match var("EMAIL") {
        Ok(email) => email,
        Err(e) => {
            warn!("Error getting EMAIL env variable : {e}");
            return None;
        }
    };

    let email_password = match var("EMAIL_PASSWORD") {
        Ok(password) => password,
        Err(e) => {
            warn!("Error getting EMAIL_PASSWORD env variable : {e}");
            return None;
        }
    };

    let smtp_client = SmtpTransport::relay(&email_smtp_server)
        .expect("Error creating SMTP client")
        .credentials(Credentials::new(email, email_password))
        .build();

    if let Err(e) = smtp_client.test_connection() {
        warn!("Error while testing connection to the SMTP server : {e}");
        return None;
    }
    info!("SMTP connection successful !");

    Some(smtp_client)
}

fn setup_cipher() -> Option<Cipher> {
    let secret_key = match var("ENCODING_KEY") {
        Ok(key) => key,
        Err(e) => {
            warn!("Error getting ENCODING_KEY env variable : {e}");
            return None;
        }
    };

    let secret_key: [u8; 32] = match secret_key.as_bytes().try_into() {
        Ok(key) => key,
        Err(e) => {
            warn!("The encryption key must be 32 bytes : {e}");
            return None;
        }
    };

    Some(Cipher::new_256(&secret_key))
}
