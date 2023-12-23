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
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::{AuthUser, InnerAuthUser},
    AppState, EventTracker, RealTimeEvent, User, UserConnection, Users, NEXT_USER_ID,
};

#[derive(Serialize)]
pub struct WsEvent {
    name: String,
    content: String,
}

#[derive(Serialize)]
pub struct NewPostNotification {
    author_username: String,
    content: String,
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
                user_id: user_followed.id,
            };
            subscribed_events.insert(event_type.clone());
            event_tracker.subscribe(event_type, sender.clone()).await;
        }

        let mut writer = users.write().await;

        if let Entry::Occupied(mut user) = writer.entry(auth_user.id) {
            user.get_mut().connections.push(sender.clone());
        } else {
            let user = User::new(sender.clone());

            writer.insert(auth_user.id, user);
        }

        drop(writer);

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

        let user = User::new(sender.clone());

        users.write().await.insert(id, user);

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
        if senders.connections.len() == 1 {
            //TODO Not sure if it's useful
            if Arc::ptr_eq(&senders.connections[0], &user) {
                entry.remove_entry();
                return;
            } else {
                warn!("User with id {id} has an unknown connection instead of the right connection...");
            }
        }
        senders
            .connections
            .retain(|sender| !Arc::ptr_eq(sender, &user));
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
