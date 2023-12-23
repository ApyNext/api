use std::{
    collections::{hash_map::Entry, HashSet},
    sync::{atomic::Ordering, Arc},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    Extension,
};

use futures_util::{stream::FuturesUnordered, SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::{AuthUser, InnerAuthUser},
    AppState, EventTracker, RealTimeEvent, UserConnection, Users, NEXT_USER_ID,
};

#[derive(Serialize)]
pub struct WsEvent {
    name: String,
    content: String,
}

impl WsEvent {
    pub fn new(name: String, content: String) -> Self {
        Self { name, content }
    }
}

#[derive(Serialize)]
pub struct NewPostNotification {
    author_username: String,
    content: String,
}

#[derive(Serialize)]
pub struct ConnectedUsersCountUpdate {
    count: usize,
}

impl ConnectedUsersCountUpdate {
    pub fn new(count: usize) -> Self {
        Self { count }
    }
}

pub async fn ws_route(
    ws: WebSocketUpgrade,
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Extension(event_tracker): Extension<EventTracker>,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, auth_user, users, event_tracker, app_state))
}

pub async fn handle_socket(
    socket: WebSocket,
    auth_user: Option<InnerAuthUser>,
    users: Users,
    event_tracker: EventTracker,
    app_state: Arc<AppState>,
) {
    let mut subscribed_events: HashSet<RealTimeEvent> = HashSet::default();

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

        //TODO to that asynchronously
        for user_followed in &users_followed {
            let event_type = RealTimeEvent::NewPostNotification {
                followed_user_id: user_followed.id,
            };
            subscribed_events.insert(event_type.clone());
            event_tracker.subscribe(event_type, sender.clone()).await;
        }

        match users.write().await.entry(auth_user.id) {
            Entry::Occupied(mut entry) => entry.get_mut().push(sender.clone()),
            Entry::Vacant(entry) => {
                entry.insert(vec![sender.clone()]);
            }
        }

        let user_count = users.read().await.keys().filter(|key| **key > -1).count();

        let event = ConnectedUsersCountUpdate::new(user_count);
        match serde_json::to_string(&event) {
            Ok(event) => {
                let event = WsEvent::new("connected_user_count_update".to_string(), event);
                match serde_json::to_string(&event) {
                    Ok(event) => {
                        event_tracker
                            .notify(RealTimeEvent::ConnectedUsersCountUpdate, event)
                            .await;
                    }
                    Err(e) => {
                        warn!("Error serializing websocket event {} : {e}", event.name);
                    }
                }
            }
            Err(e) => {
                warn!("Error serializing connected users count update event : {e}");
            }
        }

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            info!("{:?}", msg);

            //TODO handle event
        }

        disconnect(
            auth_user.id,
            sender,
            subscribed_events,
            users,
            event_tracker,
        )
        .await;
    } else {
        let id = NEXT_USER_ID.fetch_sub(1, Ordering::Relaxed);

        users.write().await.insert(id, vec![sender.clone()]);

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                return;
            };

            info!("{:?}", msg);

            //TODO handle event
        }

        disconnect(id, sender, subscribed_events, users, event_tracker).await;

        info!("Disconnected {id}");
    }
}

pub async fn remove_from_users(id: i64, users: Users, user: UserConnection) {
    if let Entry::Occupied(mut entry) = users.write().await.entry(id) {
        let senders = entry.get_mut();
        if senders.len() == 1 {
            //TODO Not sure if it's useful
            if Arc::ptr_eq(&senders[0], &user) {
                entry.remove_entry();
                return;
            }
            warn!("User with id {id} has an unknown connection instead of the right connection...");
        }
        senders.retain(|sender| !Arc::ptr_eq(sender, &user));
    } else {
        warn!("User with id {id} is not is `users`");
    };
}

pub async fn disconnect(
    id: i64,
    user: UserConnection,
    subscribed_events: HashSet<RealTimeEvent>,
    users: Users,
    event_tracker: EventTracker,
) {
    info!("Disconnecting {}", id);
    let f = FuturesUnordered::new();

    for event in subscribed_events {
        f.push(event_tracker.unsubscribe(event, user.clone()));
    }
    tokio::join!(f.collect::<Vec<()>>(), remove_from_users(id, users, user));
    info!("User {} disconnected", id);
}

pub async fn broadcast_event(users: Users, content: &str) {
    for (user_id, user) in users.write().await.iter() {
        for connection in user {
            if let Err(e) = connection
                .write()
                .await
                .send(Message::Text(content.to_string()))
                .await
            {
                warn!("Error sending event to {user_id} : {e}");
            };
        }
    }
}
