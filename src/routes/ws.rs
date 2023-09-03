use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
};

pub async fn ws_route(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

pub async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
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
