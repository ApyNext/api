use std::{
    collections::HashSet,
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
    AppState, SubscribedUser, SubscribedUsers, Subscriber, User, Users, NEXT_USER_ID,
};

#[derive(Serialize)]
pub struct SseEvent {
    pub name: String,
    pub content: String,
}

pub async fn add_subscription(
    id: i64,
    subscriber: Arc<Subscriber>,
    subscribed_users: SubscribedUsers,
) {
    if subscribed_users.read().await.contains_key(&id) {
        let subscribed_user =
            SubscribedUser::new(id, Arc::new(RwLock::new(HashSet::from([subscriber]))));
        let u = Arc::new(RwLock::new(subscribed_user));
        subscribed_users.write().await.insert(id, u);
    } else {
        let reader = subscribed_users.read().await;
        let u = reader.get(&id).unwrap();
        let reader = u.read().await;
        reader.subscribers.write().await.insert(subscriber);
    }
}

pub async fn remove_subscription(
    id: i64,
    subscriber: Arc<Subscriber>,
    subscribed_users: SubscribedUsers,
) {
    if !subscribed_users.read().await.contains_key(&id) {
        return;
    }

    let reader = subscribed_users.read().await;
    let u = reader.get(&id).unwrap();
    let reader = u.read().await;
    reader.subscribers.write().await.remove(subscriber.as_ref());
    if reader.subscribers.read().await.len() == 0 {
        subscribed_users.write().await.remove(&id);
    }
}

pub async fn ws_route(
    ws: WebSocketUpgrade,
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Extension(subscribed_users): Extension<SubscribedUsers>,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, auth_user, users, subscribed_users, app_state))
}

pub async fn handle_socket(
    socket: WebSocket,
    auth_user: Option<InnerAuthUser>,
    users: Users,
    subscribed_users: SubscribedUsers,
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

        if let std::collections::hash_map::Entry::Occupied(mut user) = writer.entry(auth_user.id) {
            user.get_mut().senders.push(sender.clone());
        } else {
            let user = User::new(following.clone(), vec![sender.clone()]);

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

        let length = users.read().await.len();

        let event = match serde_json::to_string(&SseEvent {
            name: "users_count_update".to_string(),
            content: length.to_string(),
        }) {
            Ok(e) => e,
            Err(e) => {
                warn!("{e}");
                disconnect(auth_user.id, user, users, subscribed_users).await;
                return;
            }
        };

        let msg = Message::Text(event);

        broadcast_msg(msg, users.clone()).await;

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            info!("{:?}", msg);
        }
        disconnect(auth_user.id, user, users, subscribed_users).await;
    } else {
        let id = NEXT_USER_ID.fetch_sub(1, Ordering::Relaxed);

        let user = User::new(Arc::new(RwLock::new(HashSet::new())), vec![sender]);

        users.write().await.insert(id, user);
        let length = users.read().await.len();

        let length = length.to_string();

        let event = match serde_json::to_string(&SseEvent {
            name: "users_count_update".to_string(),
            content: length,
        }) {
            Ok(e) => e,
            Err(e) => {
                warn!("{e}");
                users.write().await.remove(&id);
                return;
            }
        };

        let msg = Message::Text(event);

        broadcast_msg(msg, users.clone()).await;

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

pub async fn broadcast_msg(msg: Message, users: Users) {
    let reader = users.read().await;

    let f = FuturesUnordered::new();

    for (_, senders) in reader.iter() {
        for sender in &senders.senders {
            f.push({
                let msg = msg.clone();
                async move {
                    match sender.write().await.send(msg).await {
                        Ok(()) => {}
                        Err(e) => warn!("{e}"),
                    };
                }
            });
        }
    }

    f.collect::<Vec<()>>().await;
}

pub async fn remove_from_users(id: i64, users: Users, user: Arc<Subscriber>) {
    if let Some(senders) = users.write().await.get_mut(&id) {
        for (i, sender) in senders.senders.iter().enumerate() {
            if Arc::ptr_eq(sender, &user.sender) {
                senders.senders.remove(i);
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
    subscribed_users: SubscribedUsers,
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
