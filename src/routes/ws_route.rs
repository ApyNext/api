use std::{
    collections::{hash_map::Entry, HashSet},
    sync::{atomic::Ordering, Arc},
};

use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
    Extension,
};

use futures_util::{stream::FuturesUnordered, StreamExt};
use serde::Serialize;
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::{AuthUser, InnerAuthUser},
    AppState, EventTracker, User, Users, NEXT_USER_ID,
};

#[derive(Serialize)]
pub enum WsEvent {
    NewPostNotification {
        author_username: String,
        content: String,
    },
}

impl ToString for WsEvent {
    fn to_string(&self) -> String {
        match self {
            Self::NewPostNotification {
                author_username,
                content,
            } => json!({
                "event": "new_post_notification",
                "content": {
                    "author": author_username,
                    "content": content
                }
            })
            .to_string(),
        }
    }
}

pub async fn ws_route(
    ws: WebSocketUpgrade,
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Extension(subscribed_users): Extension<EventTracker>,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, auth_user, users, subscribed_users, app_state))
}

pub async fn handle_socket(
    socket: WebSocket,
    auth_user: Option<InnerAuthUser>,
    users: Users,
    subscribed_users: EventTracker,
    app_state: Arc<AppState>,
) {
    let (sender, mut receiver) = socket.split();

    let sender = Arc::new(RwLock::new(sender));

    if let Some(auth_user) = auth_user {
        let users_followed = match sqlx::query_as!(
            InnerAuthUser,
            r#"SELECT followed_id AS "id!" FROM follow where follower_id = $1"#,
            auth_user.id
        )
        .fetch_all(&app_state.pool)
        .await
        {
            Ok(users) => users,
            Err(e) => {
                warn!("{e}");
                return;
            }
        };
        let users_id_followed = users_followed.into_iter().map(|user| user.id);

        let users_id_followed: HashSet<i64> = users_id_followed.collect();

        let following = Arc::new(RwLock::new(users_id_followed));

        let mut writer = users.write().await;

        if let Entry::Occupied(mut user) = writer.entry(auth_user.id) {
            user.get_mut().connections.push(sender.clone());
        } else {
            let user = User::new(vec![sender.clone()]);

            writer.insert(auth_user.id, user);
        }

        drop(writer);

        let user = Subscriber::new(sender.clone(), following);

        let user = Arc::new(user);

        let user_cloned = user.clone();

        let reader = user_cloned.following.read().await;

        let f = FuturesUnordered::new();

        for id in reader.iter() {
            f.push(add_subscription(
                *id,
                user.clone(),
                subscribed_users.clone(),
            ));
        }

        f.collect::<Vec<()>>().await;

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            info!("{:?}", msg);
        }
        disconnect(auth_user.id, user, users, subscribed_users).await;
    } else {
        let id = NEXT_USER_ID.fetch_sub(1, Ordering::Relaxed);

        let user = User::new(vec![sender]);

        users.write().await.insert(id, user);

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                return;
            };

            info!("{:?}", msg);

            broadcast_msg(msg, users.clone()).await;
        }

        users.write().await.remove(&id);

        info!("Disconnected {id}");
    }
}

pub async fn remove_from_users(id: i64, users: Users, user: Arc<Subscriber>) {
    if let Some(senders) = users.write().await.get_mut(&id) {
        for (i, sender) in senders.connections.iter().enumerate() {
            if Arc::ptr_eq(sender, &user.sender) {
                senders.connections.remove(i);
                break;
            }
        }
    } else {
        warn!("User with id {id} is not is the users Vec");
    };
}

pub async fn disconnect(
    id: i64,
    user: Arc<Subscriber>,
    users: Users,
    subscribed_users: EventTracker,
) {
    info!("Disconnecting {}", id);
    let f = FuturesUnordered::new();

    for id in user.following.read().await.iter() {
        f.push(remove_subscription(
            *id,
            user.clone(),
            subscribed_users.clone(),
        ));
    }
    tokio::join!(f.collect::<Vec<()>>(), remove_from_users(id, users, user));
    info!("User {} disconnected", id);
}
