use std::{
    convert::Infallible,
    sync::{atomic::Ordering, Arc, RwLock},
};

use axum::{
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Extension,
};

use serde::Serialize;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_stream::{Stream, StreamExt};
use tracing::info;

use crate::{SubscribedUsers, Users, NEXT_USER_ID};

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

//pub fn add_subscription(id: usize, )

pub async fn sse_route(
    Extension(users): Extension<Users>,
    Extension(subscribed_users): Extension<SubscribedUsers>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    //Generate user id
    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    let (sender, receiver): (UnboundedSender<SseEvent>, UnboundedReceiver<SseEvent>) =
        mpsc::unbounded_channel::<SseEvent>();

    let sender = Arc::new(RwLock::new(sender));

    let stream = receiver.map(|result| {
        Event::default()
            .json_data(serde_json::to_string(&result).unwrap())
            .unwrap()
    });

    users.write().unwrap().insert(id, sender);

    //TODO use that when disconnected
    disconnect(id, users).await;

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn broadcast_msg(msg: Message, users: Users) {
    for (&_uid, tx) in users.read().unwrap().iter() {
        let e = SseEvent {
            name: String::from("post_notification"),
            content: serde_json::to_string(&msg).unwrap(),
        };
        tx.write().unwrap().send(e).expect("Failed to send message");
    }
}

pub async fn disconnect(id: usize, users: Users) {
    info!("Disconnecting {}", id);
    users.write().unwrap().remove(&id);
    info!("User {} disconnected", id);
}
