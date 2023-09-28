use std::sync::atomic::Ordering;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::Response,
    Extension,
};
use futures::StreamExt;
use futures_util::SinkExt;
use tokio::sync::mpsc;
use tracing::info;

use crate::{Msg, Users, NEXT_USER_ID};

pub async fn ws_route(ws: WebSocketUpgrade, Extension(users): Extension<Users>) -> Response {
    ws.on_upgrade(|websocket| handle_socket(websocket, users))
}

pub async fn handle_socket(socket: WebSocket, users: Users) {
    //Generate user id
    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    let (mut sender, mut receiver) = socket.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    users.write().unwrap().insert(id, tx);

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            sender.send(msg).await.expect("Error while sending message");
        }
        sender.close().await.unwrap();
    });
    while let Some(Ok(result)) = receiver.next().await {
        println!("{:?}", result);
        if let Ok(result) = enrich_result(result, id) {
            broadcast_msg(result, &users).await;
        }
    }

    disconnect(id, &users).await;
}

pub async fn broadcast_msg(msg: Message, users: &Users) {
    if let Message::Text(msg) = msg {
        for (&_uid, tx) in users.read().unwrap().iter() {
            tx.send(Message::Text(msg.clone()))
                .expect("Failed to send message");
        }
    }
}

pub fn enrich_result(result: Message, id: usize) -> Result<Message, serde_json::Error> {
    match result {
        Message::Text(msg) => {
            let mut msg: Msg = serde_json::from_str(&msg)?;
            msg.uid = Some(id);
            let msg = serde_json::to_string(&msg)?;
            Ok(Message::Text(msg))
        }
        _ => Ok(result),
    }
}

pub async fn disconnect(id: usize, users: &Users) {
    info!("Disconnecting {}", id);
    users.write().unwrap().remove(&id);
    info!("User {} disconnected", id);
}
