use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::{atomic::Ordering, Arc},
};

use axum::extract::ws::Message;
use futures_util::{stream::FuturesUnordered, SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    routes::ws_route::{CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME, NEW_POST_NOTIFICATION_EVENT_NAME},
    UserConnection, Users, CONNECTED_USERS_COUNT,
};

#[derive(Eq, PartialEq, Hash, Clone)]
pub enum RealTimeEvent {
    NewPostNotification { followed_user_id: i64 },
    ConnectedUsersCountUpdate,
}

impl RealTimeEvent {
    pub async fn from_client_event(client_event: ClientEvent) -> Result<Self, String> {
        match client_event.get_name() {
            "subscribe_to_event" => {
                let content = client_event.get_content();
                let Some(event_name) = content.get("name") else {
                    return Err(
                        "Le champs `name` est manquant à l'intérieur de `content`.".to_string()
                    );
                };

                let Some(event_name) = event_name.as_str() else {
                    return Err("Le champs `name` à l'intérieur de `content` doit être une chaîne de caractères.".to_string());
                };

                match event_name {
                    //no, already a route for that
                    CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME => {
                        Ok(RealTimeEvent::ConnectedUsersCountUpdate)
                    }
                    _ => Err("L'event `{event_name}` n'existe pas.".to_string()),
                }
            }
            "unsubscribe_to_event" => {
                unimplemented!()
            }
            _ => unimplemented!(),
        }
    }
}

#[derive(Default, Clone)]
pub struct EventTracker {
    events: Arc<RwLock<HashMap<RealTimeEvent, Vec<UserConnection>>>>,
}

impl EventTracker {
    pub async fn subscribe(&self, event_type: RealTimeEvent, subscriber: UserConnection) {
        //Check if the event already exists
        match self.events.write().await.entry(event_type) {
            //If it exists, add the connection to the subscribers of this event
            Entry::Occupied(mut entry) => {
                let entry = entry.get_mut();
                entry.push(subscriber);
            }
            //If it doesn't exist yet, add the event to the list of events and add the connection to it
            Entry::Vacant(e) => {
                e.insert(vec![subscriber]);
            }
        }
    }

    pub async fn unsubscribe(&self, event_type: RealTimeEvent, subscriber: UserConnection) {
        if let Entry::Occupied(mut entry) = self.events.write().await.entry(event_type) {
            let users = entry.get_mut();
            if users.len() == 1 {
                //Note sure if that's useful
                if Arc::ptr_eq(&users[0], &subscriber) {
                    entry.remove_entry();
                }
                return;
            }
            users.retain(|s| !Arc::ptr_eq(s, &subscriber));
            return;
        }
        warn!("User not subscribed to event.");
    }

    pub async fn notify(&self, event_type: RealTimeEvent, content: String) {
        if let Some(connections) = self.events.read().await.get(&event_type) {
            let f = FuturesUnordered::new();

            for connection in connections {
                f.push({
                    let content = content.clone();
                    async move {
                        if let Err(e) = connection.write().await.send(Message::Text(content)).await
                        {
                            warn!("{e}");
                        }
                    }
                });
            }

            f.collect::<Vec<()>>().await;
        }
    }

    pub async fn handle_client_event(
        &self,
        event: &str,
        sender: UserConnection,
    ) -> Result<(), String> {
        let client_event: ClientEvent = match serde_json::from_str(event) {
            Ok(e) => e,
            Err(e) => {
                warn!("Error deserializing WS event : {e}");
                return Err("Invalid JSON event".to_string());
            }
        };
        let real_time_event = match RealTimeEvent::from_client_event(client_event).await {
            Ok(e) => e,
            Err(e) => {
                warn!("Error deserializing event : {e}");
                return Err("Invalid JSON event".to_string());
            }
        };

        self.subscribe(real_time_event, sender.clone()).await;

        Ok(())
    }
}

pub struct WsEvent;

impl WsEvent {
    //TODO change content to a Post struct
    pub fn new_new_post_modification_event(author: &str, content: &str) -> serde_json::Value {
        json! ({
            "name": NEW_POST_NOTIFICATION_EVENT_NAME,
            "content": {
                "author": author,
                "content": content
            },
        })
    }
    pub fn new_connected_users_count_update_event(count: usize) -> serde_json::Value {
        json! ({
            "name": CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME,
            "content": count,
        })
    }
}

#[derive(Deserialize)]
pub struct ClientEvent {
    name: String,
    content: serde_json::Value,
}

impl ClientEvent {
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_content(&self) -> &serde_json::Value {
        &self.content
    }
}
pub async fn remove_from_users(
    id: i64,
    users: Users,
    user: UserConnection,
    event_tracker: &EventTracker,
) {
    if let Entry::Occupied(mut entry) = users.write().await.entry(id) {
        let senders = entry.get_mut();
        if senders.len() == 1 {
            //TODO Not sure if it's useful
            if Arc::ptr_eq(&senders[0], &user) {
                if id > -1 {
                    let user_count = CONNECTED_USERS_COUNT.fetch_sub(1, Ordering::Relaxed);
                    let event =
                        WsEvent::new_connected_users_count_update_event(user_count).to_string();
                    event_tracker
                        .notify(RealTimeEvent::ConnectedUsersCountUpdate, event)
                        .await;
                }
                entry.remove_entry();
                return;
            }
            warn!("User with id {id} has an unknown connection instead of the right connection...");
        }
        senders.retain(|sender| !Arc::ptr_eq(sender, &user));
    } else {
        warn!("User with id {id} is not is `users`");
    };
}

pub async fn disconnect(
    id: i64,
    user: UserConnection,
    subscribed_events: HashSet<RealTimeEvent>,
    users: Users,
    event_tracker: EventTracker,
) {
    info!("Disconnecting {}", id);
    let f = FuturesUnordered::new();

    for event in subscribed_events {
        f.push(event_tracker.unsubscribe(event, user.clone()));
    }
    tokio::join!(
        f.collect::<Vec<()>>(),
        remove_from_users(id, users, user, &event_tracker)
    );
    info!("User {} disconnected", id);
}

pub async fn broadcast_event(users: Users, content: &str) {
    for (user_id, user) in users.write().await.iter() {
        for connection in user {
            if let Err(e) = connection
                .write()
                .await
                .send(Message::Text(content.to_string()))
                .await
            {
                warn!("Error sending event to {user_id} : {e}");
            };
        }
    }
}
