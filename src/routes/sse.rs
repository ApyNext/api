use std::{
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
use tokio::sync::{RwLock, mpsc::UnboundedReceiver};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
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

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<SseEvent>();

    let sender = Arc::new(RwLock::new(sender));

    let stream = UnboundedReceiverStream::new(receiver);

    let stream = stream.map(|result| {
        Event::default()
            .json_data(&result)
            .unwrap()
    });

    users.write().await.insert(id, sender);

    let stream = stream.map(Ok::<_, Infallible>);

    //TODO use that when disconnected
    disconnect(id, users).await;

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

pub async fn disconnect(id: usize, users: Users) {
    info!("Disconnecting {}", id);
    users.write().await.remove(&id);
    info!("User {} disconnected", id);
}
