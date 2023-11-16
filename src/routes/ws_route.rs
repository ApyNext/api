use std::{collections::HashSet, sync::Arc};

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
    AppState, SubscribedUser, SubscribedUsers, User, Users,
};

#[derive(Serialize)]
pub struct SseEvent {
    pub name: String,
    pub content: String,
}

pub async fn add_subscription(id: i64, subscriber: Arc<User>, subscribed_users: SubscribedUsers) {
    if subscribed_users.read().await.contains_key(&id) {
        let u = Arc::new(RwLock::new(SubscribedUser {
            id,
            subscribers: Arc::new(RwLock::new(HashSet::from([subscriber]))),
        }));
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
    subscriber: Arc<User>,
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

    match auth_user {
        Some(auth_user) => {
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

            let mut writer = users.write().await;

            if writer.contains_key(&auth_user.id) {
                writer.get_mut(&auth_user.id).unwrap().push(sender.clone());
            } else {
                writer.insert(auth_user.id, Vec::from_iter([sender.clone()].into_iter()));
            }

            drop(writer);

            let users_id_followed = users_followed.into_iter().map(|user| user.id).into_iter();

            let users_id_followed = HashSet::from_iter(users_id_followed);

            let user = User {
                sender: sender.clone(),
                following: Arc::new(RwLock::new(users_id_followed)),
            };

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
                let msg = if let Ok(msg) = msg {
                    msg
                } else {
                    break;
                };

                info!("{:?}", msg);
            }
            disconnect(auth_user.id, user, users, subscribed_users).await;
        }
        None => {
            while let Some(msg) = receiver.next().await {
                let msg = if let Ok(msg) = msg {
                    msg
                } else {
                    return;
                };

                info!("{:?}", msg);
            }
        }
    }
}

pub async fn broadcast_msg(msg: Message, users: Users) {
    let reader = users.read().await;

    let f = FuturesUnordered::new();

    for (_, senders) in reader.iter() {
        for sender in senders.iter() {
            f.push({
                let msg = msg.clone();
                async move {
                    match sender.write().await.send(msg).await {
                        Ok(_) => {}
                        Err(e) => warn!("{e}"),
                    };
                }
            });
        }
    }

    f.collect::<Vec<()>>().await;
}

pub async fn remove_from_users(id: i64, users: Users, user: Arc<User>) {
    match users.write().await.get_mut(&id) {
        Some(senders) => {
            for (i, sender) in senders.iter().enumerate() {
                if Arc::ptr_eq(sender, &user.sender) {
                    senders.remove(i);
                    break;
                }
            }
        }
        None => warn!("User with id {id} is not is the users Vec"),
    };
}

pub async fn disconnect(id: i64, user: Arc<User>, users: Users, subscribed_users: SubscribedUsers) {
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
