use std::sync::{atomic::Ordering, Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    Extension,
};

use futures_util::StreamExt;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::{AuthUser, InnerAuthUser},
    utils::{
        authentification::authentificate,
        real_time_event_management::{EventTracker, RealTimeEvent, UserConnection, Users, WsEvent},
    },
    AppState, NEXT_NOT_CONNECTED_USER_ID,
};

pub async fn ws_route(
    ws: WebSocketUpgrade,

    Extension(users): Extension<Users>,
    Extension(event_tracker): Extension<EventTracker>,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, users, event_tracker, app_state))
}

pub async fn handle_socket(
    socket: WebSocket,

    users: Users,
    event_tracker: EventTracker,
    app_state: Arc<AppState>,
) {
    let (sender, mut receiver) = socket.split();

    let AuthUser(auth_user) = loop {
        let Some(msg) = receiver.next().await else {
            warn!("Client didn't send the authentification token.");
            continue;
        };
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Client didn't send a valid message : {e}");
                continue;
            }
        };

        let Message::Text(token) = msg else {
            warn!("Client sent a non-text event.");
            continue;
        };

        let auth_user = match authentificate(app_state.clone(), &token).await {
            Ok(auth_user) => auth_user,
            Err(_) => {
                warn!("Invalid credentials");
                continue;
            }
        };

        break auth_user;
    };

    let user = Arc::new(RwLock::new(UserConnection::new(sender)));

    if let Some(auth_user) = auth_user {
        info!("User with id {} has connected.", auth_user.id);
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

        //Perhaps do that asynchronously
        for user_followed in &users_followed {
            let event_type = RealTimeEvent::NewPostNotification {
                followed_user_id: user_followed.id,
            };
            event_tracker.subscribe(event_type, user.clone()).await;
        }

        event_tracker
            .add_to_users(auth_user.id, users.clone(), user.clone())
            .await;

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            if let Message::Text(text) = msg {
                info!("{} sent the WS event `{text}`", auth_user.id);

                if let Err(e) = event_tracker.handle_client_event(&text, user.clone()).await {
                    if let Err(e) = user
                        .write()
                        .await
                        .send_text_event(WsEvent::new_error(&e).to_string())
                        .await
                    {
                        warn!("Error sending error to client : {e}");
                    };
                };
            }
        }

        event_tracker.disconnect(auth_user.id, user, users).await;
    } else {
        let id = NEXT_NOT_CONNECTED_USER_ID.fetch_sub(1, Ordering::Relaxed);

        info!("User with random id {id} has connected.");

        users.write().await.insert(id, vec![user.clone()]);

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            if let Message::Text(text) = msg {
                info!("Not connected user {id} sent the WS event `{text}`");

                if let Err(e) = event_tracker.handle_client_event(&text, user.clone()).await {
                    if let Err(e) = user
                        .write()
                        .await
                        .send_text_event(WsEvent::new_error(&e).to_string())
                        .await
                    {
                        warn!("Error sending error to client : {e}");
                    };
                }
            }
        }

        event_tracker.disconnect(id, user, users).await;

        info!("Disconnected {id}");
    }
}
