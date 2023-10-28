use std::{
    collections::HashSet,
    convert::Infallible,
    sync::{atomic::Ordering, Arc},
};

use axum::{
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Extension,
};

use futures_util::Stream;
use serde::Serialize;
use tokio::sync::RwLock;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tracing::info;

use crate::{Following, SubscribedUser, SubscribedUsers, User, Users, NEXT_USER_ID};

#[derive(Serialize)]
pub struct Message {
    author: i64,
    content: String,
}

#[derive(Serialize)]
pub struct SseEvent {
    name: String,
    content: String,
}

pub async fn add_subscription(id: usize, subscriber: Arc<User>, subscribed_users: SubscribedUsers) {
    if subscribed_users.read().await.contains_key(&id) {
        let u = Arc::new(RwLock::new(SubscribedUser {
            id,
            subscribers: Arc::new(RwLock::new(HashSet::from([subscriber]))),
        }));
        subscribed_users.write().await.insert(id, u);
    } else {
        let u = subscribed_users.read().await.get(&id).unwrap();
        let reader = u.read().await;
        reader.subscribers.write().await.insert(subscriber);
    }
}

pub async fn remove_subscription(
    id: usize,
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

pub async fn sse_route(
    Extension(users): Extension<Users>,
    Extension(subscribed_users): Extension<SubscribedUsers>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    //Generate user id
    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<SseEvent>();

    let sender = Arc::new(RwLock::new(sender));

    let stream = UnboundedReceiverStream::new(receiver);

    let stream = stream.map(|result| Event::default().json_data(&result).unwrap());

    users.write().await.insert(id, sender.clone());

    //TODO get following

    let user = User {
        sender,
        following: Following::default(),
    };

    let user = Arc::new(user);

    for id in user.following.read().await.iter() {
        add_subscription(*id, user.clone(), subscribed_users.clone()).await;
    }

    let stream = stream.map(Ok::<_, Infallible>);

    //TODO use that when disconnected
    disconnect(id, user, users, subscribed_users).await;

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn broadcast_msg(msg: Message, users: Users) {
    for (_, tx) in users.read().await.iter() {
        let e = SseEvent {
            name: String::from("post_notification"),
            content: serde_json::to_string(&msg).unwrap(),
        };
        tx.write().await.send(e).expect("Failed to send message");
    }
}

pub async fn disconnect(
    id: usize,
    user: Arc<User>,
    users: Users,
    subscribed_users: SubscribedUsers,
) {
    info!("Disconnecting {}", id);
    users.write().await.remove(&id);
    for id in user.following.read().await.iter() {
        remove_subscription(*id, user.clone(), subscribed_users.clone()).await;
    }
    info!("User {} disconnected", id);
}
