mod extractors;
mod middleware;
mod routes;
mod structs;
mod utils;

use std::collections::{HashMap, HashSet};
use std::env::var;
use std::hash::Hash;
use std::net::SocketAddr;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::{
    middleware as axum_middleware,
    routing::{get, post},
};
use axum::{Extension, Router};
use futures_util::stream::SplitSink;
use libaes::Cipher;
use routes::follow_user_route::follow_user_route;
use routes::ws_route::ws_route;

use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::utils::delete_not_activated_expired_accounts::delete_not_activated_expired_accounts;
use hyper::header::HeaderValue;
use hyper::http::Method;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use middleware::logger_middleware::logger_middleware;
use routes::a2f_login_route::a2f_login_route;
use routes::email_confirm_route::email_confirm_route;
use routes::login_route::login_route;
use routes::ok_route::ok_route;
use routes::register_route::register_route;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub struct AppState {
    pool: PgPool,
    smtp_client: SmtpTransport,
    cipher: Cipher,
}

pub struct SubscribedUser {
    id: i64,
    subscribers: Subscribers,
}

pub struct User {
    sender: UserConnection,
    following: Following,
}

impl Eq for User {}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.following, &other.following)
    }
}

impl Hash for User {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.following).hash(state);
    }
}

const FRONT_URL: &str = "https://apynext.creativeblogger.org";
type Subscribers = Arc<RwLock<HashSet<Arc<User>>>>;
type Following = Arc<RwLock<HashSet<i64>>>;
type SubscribedUsers = Arc<RwLock<HashMap<i64, Arc<RwLock<SubscribedUser>>>>>;
type UserConnection = Arc<RwLock<SplitSink<WebSocket, Message>>>;
type Users = Arc<RwLock<HashMap<i64, Vec<UserConnection>>>>;
static NEXT_USER_ID: AtomicI64 = AtomicI64::new(-1);

#[tokio::main]
async fn main() {
    let database_url = match var("DATABASE_URL") {
        Ok(url) => url,
        Err(e) => {
            warn!("Error getting DATABASE_URL env variable : {e}");
            return;
        }
    };

    let secret_key = match var("ENCODING_KEY") {
        Ok(key) => key,
        Err(e) => {
            warn!("Error getting ENCODING_KEY env variable : {e}");
            return;
        }
    };

    let secret_key: [u8; 32] = match secret_key.as_bytes().try_into() {
        Ok(key) => key,
        Err(e) => {
            warn!("The encryption key must be 32 bytes : {e}");
            return;
        }
    };

    let front_url = match var("FRONT_URL") {
        Ok(url) => url,
        Err(e) => {
            warn!("Error getting FRONT_URL env variable : {e}");
            return;
        }
    };

    let front_url = match front_url.parse::<HeaderValue>() {
        Ok(url) => url,
        Err(e) => {
            warn!("FRONT_URL is an invalid URL : {e}");
            return;
        }
    };

    let email_smtp_server = match var("EMAIL_SMTP_SERVER") {
        Ok(server_addr) => server_addr,
        Err(e) => {
            warn!("Error getting EMAIL_SMTP_SERVER env variable : {e}");
            return;
        }
    };

    let email = match var("EMAIL") {
        Ok(email) => email,
        Err(e) => {
            warn!("Error getting EMAIL env variable : {e}");
            return;
        }
    };

    let email_password = match var("EMAIL_PASSWORD") {
        Ok(password) => password,
        Err(e) => {
            warn!("Error getting EMAIL_PASSWORD env variable : {e}");
            return;
        }
    };

    let pool = match PgPoolOptions::new().connect(&database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            warn!("Error connecting to the DB : {e}");
            return;
        }
    };

    drop(database_url);

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let smtp_client = SmtpTransport::relay(&email_smtp_server)
        .unwrap()
        .credentials(Credentials::new(email, email_password))
        .build();

    drop(email_smtp_server);

    if smtp_client.test_connection().unwrap() {
        info!("Connexion SMTP effectuée avec succès !");
    }

    let app_state = Arc::new(AppState {
        pool: pool.clone(),
        smtp_client,
        cipher: Cipher::new_256(&secret_key),
    });

    //Drop secret_key as it's no longer needed
    let _ = secret_key;

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_origin(front_url);

    let router = Router::new()
        .route("/", get(ok_route))
        .route("/register", post(register_route))
        .route("/register/email_confirm", post(email_confirm_route))
        .route("/login", post(login_route))
        .route("/login/a2f", post(a2f_login_route))
        .route("/ws", get(ws_route))
        .route("/@:id/follow", post(follow_user_route))
        .layer(cors)
        .layer(axum_middleware::from_fn(logger_middleware))
        .layer(Extension(Users::default()))
        .layer(Extension(SubscribedUsers::default()))
        .with_state(app_state);
    let serve_router = axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

    tokio::select! {
        _ = delete_not_activated_expired_accounts(&pool) => {
            warn!("This should never happen");
        },
        _ = serve_router => {}
    };
}
