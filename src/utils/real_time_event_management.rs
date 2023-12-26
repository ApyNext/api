use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{atomic::Ordering, Arc},
};

use axum::extract::ws::Message;
use futures_util::{stream::FuturesUnordered, SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{UserConnection, Users, CONNECTED_USERS_COUNT};

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub enum RealTimeEvent {
    NewPostNotification { followed_user_id: i64 },
    ConnectedUsersCountUpdate,
}

#[derive(Default, Clone)]
pub struct EventTracker {
    events: Arc<RwLock<HashMap<RealTimeEvent, Vec<Arc<RwLock<UserConnection>>>>>>,
}

pub const NEW_POST_NOTIFICATION_EVENT_NAME: &str = "new_post_notification";
pub const CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME: &str = "connected_users_count_update";

impl EventTracker {
    pub async fn subscribe(
        &self,
        event_type: RealTimeEvent,
        subscriber: Arc<RwLock<UserConnection>>,
    ) {
        subscriber
            .write()
            .await
            .subscribed_events
            .insert(event_type.clone());
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

    pub async fn unsubscribe(
        &self,
        event_type: RealTimeEvent,
        subscriber: Arc<RwLock<UserConnection>>,
    ) {
        if !subscriber
            .write()
            .await
            .subscribed_events
            .remove(&event_type)
        {
            warn!("Event {event_type:?} was not present in the list of events.");
        };
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
                        if let Err(e) = connection
                            .write()
                            .await
                            .sender
                            .send(Message::Text(content))
                            .await
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
        client_event_text: &str,
        sender: Arc<RwLock<UserConnection>>,
    ) -> Result<(), String> {
        let client_event: ClientEvent = match serde_json::from_str(client_event_text) {
            Ok(e) => e,
            Err(e) => {
                warn!("Error deserializing WS event : {e}");
                return Err("Event JSON invalide.".to_string());
            }
        };

        let content = client_event.content;
        let Some(inner_event_name) = content.get("event") else {
            return Err("Le champs `event` est manquant à l'intérieur de `content`.".to_string());
        };

        let Some(event_name) = inner_event_name.as_str() else {
            return Err(
                "Le champs `event` à l'intérieur de `content` doit être une chaîne de caractères."
                    .to_string(),
            );
        };

        let event = match event_name {
            //no, already a route for that
            CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME => RealTimeEvent::ConnectedUsersCountUpdate,
            event => return Err(format!("L'event `{event}` n'existe pas.")),
        };

        match client_event.action.as_str() {
            "subscribe_to_event" => self.subscribe(event, sender).await,
            "unsubscribe_to_event" => self.unsubscribe(event, sender).await,
            action => return Err(format!("L'action `{action}` n'existe pas.")),
        }

        Ok(())
    }

    pub async fn remove_from_users(
        &self,
        id: i64,
        users: Users,
        user: Arc<RwLock<UserConnection>>,
    ) {
        if let Entry::Occupied(mut entry) = users.write().await.entry(id) {
            let user_connections = entry.get_mut();
            if user_connections.len() == 1 {
                //TODO Not sure if it's useful
                if Arc::ptr_eq(&user_connections[0], &user) {
                    if id > -1 {
                        let user_count = CONNECTED_USERS_COUNT.fetch_sub(1, Ordering::Relaxed);
                        let event =
                            WsEvent::new_connected_users_count_update_event(user_count).to_string();
                        self.notify(RealTimeEvent::ConnectedUsersCountUpdate, event)
                            .await;
                    }
                    entry.remove_entry();
                    return;
                }
                warn!("User with id {id} has an unknown connection instead of the right connection...");
            }
            user_connections.retain(|connection| !Arc::ptr_eq(connection, &user));
        } else {
            warn!("User with id {id} is not is `users`");
        };
    }

    pub async fn disconnect(self, id: i64, user: Arc<RwLock<UserConnection>>, users: Users) {
        info!("Disconnecting {}", id);
        let f = FuturesUnordered::new();

        let subscribed_events = user.read().await.subscribed_events.clone();

        for event in subscribed_events {
            f.push(self.unsubscribe(event, user.clone()));
        }
        tokio::join!(
            f.collect::<Vec<()>>(),
            self.remove_from_users(id, users, user)
        );
        info!("User {} disconnected", id);
    }
}

pub struct WsEvent;

impl WsEvent {
    //TODO change content to a Post struct
    pub fn new_new_post_modification_event(author: &str, content: &str) -> serde_json::Value {
        json! ({
            "event": NEW_POST_NOTIFICATION_EVENT_NAME,
            "content": {
                "author": author,
                "content": content
            },
        })
    }
    pub fn new_connected_users_count_update_event(count: usize) -> serde_json::Value {
        json! ({
            "event": CONNECTED_USERS_COUNT_UPDATE_EVENT_NAME,
            "content": count,
        })
    }
}

#[derive(Deserialize)]
pub struct ClientEvent {
    action: String,
    content: serde_json::Value,
}

//TODO use it
pub async fn broadcast_event(users: Users, content: &str) {
    for (user_id, user) in users.write().await.iter() {
        for connection in user {
            if let Err(e) = connection
                .write()
                .await
                .sender
                .send(Message::Text(content.to_string()))
                .await
            {
                warn!("Error sending event to {user_id} : {e}");
            };
        }
    }
}
