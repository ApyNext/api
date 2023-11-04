use std::{collections::HashSet, convert::Infallible, sync::Arc};

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Extension,
};

use futures_util::{stream::FuturesUnordered, Stream};
use serde::Serialize;
use tokio::sync::{mpsc::unbounded_channel, RwLock};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tracing::{info, warn};

use crate::{
    extractors::auth_extractor::{AuthUser, InnerAuthUser},
    utils::app_error::AppError,
    AppState, SubscribedUser, SubscribedUsers, User, Users,
};

#[derive(Serialize)]
pub struct Message {
    pub author: i64,
    pub content: String,
}

#[derive(Serialize)]
pub struct SseEvent {
    pub name: String,
    pub content: String,
}

pub async fn add_subscription(id: i64, subscriber: Arc<User>, subscribed_users: SubscribedUsers) {
    if subscribed_users.read().await.contains_key(&id) {
        let u = Arc::new(RwLock::new(SubscribedUser {
            id,
            subscribers: Arc::new(RwLock::new(HashSet::from([subscriber]))),
        }));
        subscribed_users.write().await.insert(id, u);
    } else {
        let reader = subscribed_users.read().await;
        let u = reader.get(&id).unwrap();
        let reader = u.read().await;
        reader.subscribers.write().await.insert(subscriber);
    }
}

pub async fn remove_subscription(
    id: i64,
    subscriber: Arc<User>,
    subscribed_users: SubscribedUsers,
) {
    if !subscribed_users.read().await.contains_key(&id) {
        return;
    }

    let reader = subscribed_users.read().await;
    let u = reader.get(&id).unwrap();
    let reader = u.read().await;
    reader.subscribers.write().await.remove(subscriber.as_ref());
    if reader.subscribers.read().await.len() == 0 {
        subscribed_users.write().await.remove(&id);
    }
}

pub async fn sse_route(
    AuthUser(auth_user): AuthUser,
    Extension(users): Extension<Users>,
    Extension(subscribed_users): Extension<SubscribedUsers>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    if let None = auth_user {
        info!("ah embêtant");
    }
    let auth_user = match auth_user {
        Some(user) => user,
        None => return Err(AppError::YouHaveToBeConnectedToPerformThisAction),
    };

    let (sender, receiver) = unbounded_channel::<Arc<SseEvent>>();

    let cloned_sender = sender.clone();

    let sender = Arc::new(RwLock::new(sender));

    let stream = UnboundedReceiverStream::new(receiver);

    let stream = stream.map(|sse_event| Ok(Event::default().json_data(sse_event).unwrap()));

    let mut writer = users.write().await;

    if writer.contains_key(&auth_user.id) {
        writer.get_mut(&auth_user.id).unwrap().push(sender.clone());
    } else {
        writer.insert(auth_user.id, Vec::from_iter([sender.clone()].into_iter()));
    }

    drop(writer);

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
            return Err(AppError::InternalServerError);
        }
    };

    let users_id_followed = users_followed.into_iter().map(|user| user.id).into_iter();

    let users_id_followed = HashSet::from_iter(users_id_followed);

    let user = User {
        sender: sender.clone(),
        following: Arc::new(RwLock::new(users_id_followed)),
    };

    let user = Arc::new(user);

    let user_cloned = user.clone();

    let reader = user_cloned.following.read().await;

    let f = FuturesUnordered::new();

    for id in reader.iter() {
        f.push(add_subscription(
            *id,
            user.clone(),
            subscribed_users.clone(),
        ));
    }

    f.collect::<Vec<()>>().await;

    tokio::spawn(async move {
        cloned_sender.closed().await;
        disconnect(auth_user.id, user, users, subscribed_users).await;
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub async fn broadcast_msg(msg: Message, users: Users) {
    let e = Arc::new(SseEvent {
        name: String::from("message"),
        content: serde_json::to_string(&msg).unwrap(),
    });

    let reader = users.read().await;

    let f = FuturesUnordered::new();

    for (_, senders) in reader.iter() {
        for sender in senders.iter() {
            f.push({
                let e = e.clone();
                async move {
                    sender
                        .write()
                        .await
                        .send(e)
                        .expect("Failed to send message");
                }
            });
        }
    }

    f.collect::<Vec<()>>().await;
}

pub async fn disconnect(id: i64, user: Arc<User>, users: Users, subscribed_users: SubscribedUsers) {
    info!("Disconnecting {}", id);
    let f = FuturesUnordered::new();

    let mut writer = users.write().await;
    match writer.get_mut(&id) {
        Some(senders) => for sender in senders.iter() {},
        None => warn!("User with id {id} is not is the users Vec"),
    }
    for id in user.following.read().await.iter() {
        f.push(remove_subscription(
            *id,
            user.clone(),
            subscribed_users.clone(),
        ));
    }
    f.collect::<Vec<()>>().await;
    info!("User {} disconnected", id);
}
