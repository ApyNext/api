use std::sync::atomic::Ordering;

use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
    Extension,
};
use futures::StreamExt;

use crate::{Users, NEXT_USER_ID};

pub async fn ws_route(ws: WebSocketUpgrade, Extension(users): Extension<Users>) -> Response {
    ws.on_upgrade(|websocket| handle_socket(websocket, users))
}

pub async fn handle_socket(socket: WebSocket, users: Users) {
    //Generate user id
    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = receiver.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            //TODO disconnected
            return;
        };

        if socket.send(msg).await.is_err() {
            //TODO disconnected
            return;
        }
    }
}
