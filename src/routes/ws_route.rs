use std::{
    collections::{hash_map::Entry, HashSet},
    sync::{atomic::Ordering, Arc},
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    Extension,
};

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::{AuthUser, InnerAuthUser},
    utils::real_time_event_management::{disconnect, EventTracker, RealTimeEvent, WsEvent},
    AppState, Users, CONNECTED_USERS_COUNT, NEXT_USER_ID,
};

pub const NEW_POST_NOTIFICATION_EVENT_NAME: &str = "new_post_notification";
pub const CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME: &str = "connected_users_count_update";

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
    let mut subscribed_events: HashSet<RealTimeEvent> = HashSet::default();

    let (sender, mut receiver) = socket.split();

    let sender = Arc::new(RwLock::new(sender));

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

        //TODO perhaps do that asynchronously
        for user_followed in &users_followed {
            let event_type = RealTimeEvent::NewPostNotification {
                followed_user_id: user_followed.id,
            };
            subscribed_events.insert(event_type.clone());
            event_tracker.subscribe(event_type, sender.clone()).await;
        }

        let mut writer = users.write().await;

        match writer.entry(auth_user.id) {
            Entry::Occupied(mut entry) => entry.get_mut().push(sender.clone()),
            Entry::Vacant(entry) => {
                entry.insert(vec![sender.clone()]);
                let user_count = CONNECTED_USERS_COUNT.fetch_add(1, Ordering::Relaxed);
                let event = WsEvent::new_connected_users_count_update_event(user_count).to_string();
                event_tracker
                    .notify(RealTimeEvent::ConnectedUsersCountUpdate, event)
                    .await;
            }
        }

        drop(writer);

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                break;
            };

            if let Message::Text(text) = msg {
                if let Err(e) = event_tracker
                    .handle_client_event(&text, sender.clone())
                    .await
                {
                    if let Err(e) = sender
                        .write()
                        .await
                        .send(Message::Text(
                            json!({
                                "name": "error",
                                "content": e
                            })
                            .to_string(),
                        ))
                        .await
                    {
                        warn!("Error sending error to client : {e}");
                    };
                    continue;
                };
            }
        }

        disconnect(
            auth_user.id,
            sender,
            subscribed_events,
            users,
            event_tracker,
        )
        .await;
    } else {
        let id = NEXT_USER_ID.fetch_sub(1, Ordering::Relaxed);

        users.write().await.insert(id, vec![sender.clone()]);

        while let Some(msg) = receiver.next().await {
            let Ok(msg) = msg else {
                return;
            };

            info!("{:?}", msg);

            //TODO handle event
        }

        disconnect(id, sender, subscribed_events, users, event_tracker).await;

        info!("Disconnected {id}");
    }
}
