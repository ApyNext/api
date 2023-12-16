mod extractors;
mod middleware;
mod routes;
mod structs;
mod utils;

use std::collections::hash_map::Entry;
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
use routes::ws_route::{ws_route, WsEvent};

use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::utils::delete_not_activated_expired_accounts::delete_not_activated_expired_accounts;
use hyper::header::{HeaderValue, OccupiedEntry};
use hyper::http::Method;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use middleware::logger::logger;
use routes::a2f_login_route::a2f_login_route;
use routes::email_confirm_route::email_confirm_route;
use routes::login_route::login_route;
use routes::ok_route::ok_route;
use routes::register_route::register_route;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};

pub struct AppState {
    pool: PgPool,
    smtp_client: SmtpTransport,
    cipher: Cipher,
}

#[derive(Eq, PartialEq, Hash)]
pub enum RealTimeEvent {
    //with the ID of the followed user
    NewPostNotification(i64),
}

#[derive(Default, Clone)]
pub struct EventTracker {
    events: Arc<RwLock<HashMap<RealTimeEvent, Vec<UserConnection>>>>,
}

impl EventTracker {
    pub async fn subscribe(&self, event_type: RealTimeEvent, subscriber: UserConnection) {
        match self.events.write().await.entry(event_type) {
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                entry.push(subscriber);
            }
            Entry::Vacant(e) => {
                e.insert(vec![subscriber]);
            }
        }
    }

    pub async fn unsubscribe(&self, event_type: RealTimeEvent, subscriber: UserConnection) {
        if let Entry::Occupied(mut entry) = self.events.write().await.entry(event_type) {
            let users = entry.get_mut();
            if users.len() == 1 {
                entry.remove_entry();
                return;
            }
            users.retain(|s| !Arc::ptr_eq(s, &subscriber));
        }
    }

    pub async fn notify(&self, event_type: RealTimeEvent, content: String) {
        if let Some(connections) = self.events.read().await.get(&event_type) {
            let event = match event_type {
                RealTimeEvent::NewPostNotification(id) => {
                    WsEvent::new("new_post_notification".to_string(), format!(""))
                }
            };
            for connection in connections {}
        }
    }
}

pub struct UserConnection {
    subscribed_events: HashSet<RealTimeEvent>,
    sender: SplitSink<WebSocket, Message>,
}

pub struct User {
    connections: Vec<Arc<RwLock<UserConnection>>>,
}

impl User {
    fn new(connection: Arc<RwLock<UserConnection>>) -> Self {
        User {
            connections: vec![connection],
        }
    }
}

const FRONT_URL: &str = env!("FRONT_URL");
type Users = Arc<RwLock<HashMap<i64, User>>>;
static NEXT_USER_ID: AtomicI64 = AtomicI64::new(-1);

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
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
        .expect("Error while creating SMTP client")
        .credentials(Credentials::new(email, email_password))
        .build();

    drop(email_smtp_server);

    if let Err(e) = smtp_client.test_connection() {
        warn!("Error while testing connection to the SMTP server : {e}");
        return;
    }
    info!("SMTP connection successful !");

    let app_state = Arc::new(AppState {
        pool: pool.clone(),
        smtp_client,
        cipher: Cipher::new_256(&secret_key),
    });

    //Drop secret_key as it's no longer needed
    let _ = secret_key;

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_origin(front_url)
        .allow_headers(Any);

    let router = Router::new()
        .route("/", get(ok_route))
        .route("/register", post(register_route))
        .route("/register/email_confirm", post(email_confirm_route))
        .route("/login", post(login_route))
        .route("/login/a2f", post(a2f_login_route))
        .route("/ws", get(ws_route))
        .route("/@:id/follow", post(follow_user_route))
        .layer(cors)
        .layer(axum_middleware::from_fn(logger))
        .layer(Extension(Users::default()))
        .layer(Extension(EventTracker::default()))
        .with_state(app_state);
    let serve_router = axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

    tokio::select! {
        () = delete_not_activated_expired_accounts(&pool) => {
            warn!("This should never happen");
        },
        _ = serve_router => {}
    };
}
