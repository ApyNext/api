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
    utils::real_time_event_management::{
        EventTracker, RealTimeEvent, UserConnection, Users, WsEvent,
    },
    AppState, NEXT_NOT_CONNECTED_USER_ID,
};

pub async fn ws_route(
    ws: WebSocketUpgrade,
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Extension(event_tracker): Extension<EventTracker>,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, auth_user, users, event_tracker, app_state))
}

pub async fn handle_socket(
    socket: WebSocket,
    auth_user: Option<InnerAuthUser>,
    users: Users,
    event_tracker: EventTracker,
    app_state: Arc<AppState>,
) {
    let (sender, mut receiver) = socket.split();

    let user = Arc::new(RwLock::new(UserConnection::new(sender)));

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
