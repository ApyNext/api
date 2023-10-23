use std::sync::{atomic::Ordering, Arc, RwLock};

use axum::{
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Extension,
};

use futures_channel::mpsc;
use futures_util::{Stream, StreamExt};
use serde::Serialize;
use tracing::info;

use crate::{SubscribedUsers, Users, NEXT_USER_ID};

#[derive(Serialize)]
pub struct Message {
    author: i64,
    content: String,
}

//pub fn add_subscription(id: usize, )

pub async fn sse_route(
    Extension(users): Extension<Users>,
    Extension(subscribed_users): Extension<SubscribedUsers>,
) -> Sse<impl Stream<Item = Event>> {
    //Generate user id
    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    let (sender, receiver) = mpsc::unbounded::<Event>();

    let sender = Arc::new(RwLock::new(sender));

    let stream = receiver.map(|result| result);

    users.write().unwrap().insert(id, sender);

    //TODO use that when disconnected
    disconnect(id, users).await;

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn broadcast_msg(msg: Message, users: Users) {
    for (&_uid, tx) in users.read().unwrap().iter() {
        let e = Event::default()
            .json_data(serde_json::to_string(&msg).unwrap())
            .unwrap();
        tx.write().unwrap().send(e).expect("Failed to send message");
    }
}

pub async fn disconnect(id: usize, users: Users) {
    info!("Disconnecting {}", id);
    users.write().unwrap().remove(&id);
    info!("User {} disconnected", id);
}
