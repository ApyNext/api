mod extractors;
mod middleware;
mod routes;
mod structs;
mod utils;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
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
use futures_util::stream::{FuturesUnordered, SplitSink};
use futures_util::{SinkExt, StreamExt};
use libaes::Cipher;
use routes::follow_user_route::follow_user_route;
use routes::ws_route::{
    ws_route, ClientEvent, CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME,
    NEW_POST_NOTIFICATION_EVENT_NAME,
};

use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::utils::delete_not_activated_expired_accounts::delete_not_activated_expired_accounts;
use hyper::header::HeaderValue;
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

#[derive(Eq, PartialEq, Hash, Clone)]
pub enum RealTimeEvent {
    NewPostNotification { followed_user_id: i64 },
    ConnectedUsersCountUpdate,
}

impl RealTimeEvent {
    pub async fn from_client_event(client_event: ClientEvent) -> Result<Self, String> {
        match client_event.get_name() {
            "subscribe_to_event" => {
                let content = client_event.get_content();
                let Some(event_name) = content.get("name") else {
                    return Err(
                        "Le champs `name` est manquant à l'intérieur de `content`.".to_string()
                    );
                };

                let Some(event_name) = event_name.as_str() else {
                    return Err("Le champs `name` à l'intérieur de `content` doit être une chaîne de caractères".to_string());
                };

                match event_name {
                    //no, already a route for that
                    CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME => {
                        Ok(RealTimeEvent::ConnectedUsersCountUpdate)
                    }
                    _ => unimplemented!(),
                }
            }
            "unsubscribe_to_event" => {
                unimplemented!()
            }
            _ => unimplemented!(),
        }
    }
}

#[derive(Default, Clone)]
pub struct EventTracker {
    events: Arc<RwLock<HashMap<RealTimeEvent, Vec<UserConnection>>>>,
}

impl EventTracker {
    pub async fn subscribe(&self, event_type: RealTimeEvent, subscriber: UserConnection) {
        //Check if the event already exists
        match self.events.write().await.entry(event_type) {
            //If it exists, add the connection to the subscribers of this event
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                entry.push(subscriber);
            }
            //If it doesn't exist yet, add the event to the list of events and add the connection to it
            Entry::Vacant(e) => {
                e.insert(vec![subscriber]);
            }
        }
    }

    pub async fn unsubscribe(&self, event_type: RealTimeEvent, subscriber: UserConnection) {
        if let Entry::Occupied(mut entry) = self.events.write().await.entry(event_type) {
            let users = entry.get_mut();
            if users.len() == 1 {
                //Note sure if that's useful
                if Arc::ptr_eq(&users[0], &subscriber) {
                    entry.remove_entry();
                }
                return;
            }
            users.retain(|s| !Arc::ptr_eq(s, &subscriber));
            return;
        }
        warn!("User not subscribed to event.");
    }

    pub async fn notify(&self, event_type: RealTimeEvent, content: String) {
        if let Some(connections) = self.events.read().await.get(&event_type) {
            let f = FuturesUnordered::new();

            for connection in connections {
                f.push({
                    let content = content.clone();
                    async move {
                        if let Err(e) = connection.write().await.send(Message::Text(content)).await
                        {
                            warn!("{e}");
                        }
                    }
                });
            }

            f.collect::<Vec<()>>().await;
        }
    }
}

const FRONT_URL: &str = env!("FRONT_URL");
type UserConnection = Arc<RwLock<SplitSink<WebSocket, Message>>>;
type Users = Arc<RwLock<HashMap<i64, Vec<UserConnection>>>>;
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
        .expect("Error creating SMTP client")
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
        .route("/@:username/follow", post(follow_user_route))
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
