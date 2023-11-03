use std::{
    collections::HashSet,
    convert::Infallible,
    sync::{atomic::Ordering, Arc},
};

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
    AppState, SubscribedUser, SubscribedUsers, User, Users, NEXT_USER_ID,
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
    //Generate user id
    let random_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    let (sender, receiver) = unbounded_channel::<Arc<SseEvent>>();

    let cloned_sender = sender.clone();

    let sender = Arc::new(RwLock::new((auth_user.id, sender)));

    let stream = UnboundedReceiverStream::new(receiver);

    let stream = stream.map(|sse_event| Ok(Event::default().json_data(sse_event).unwrap()));

    users.write().await.insert(random_id, sender.clone());

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
        disconnect(random_id, user, users, subscribed_users).await;
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

    for (_, sender) in reader.iter() {
        f.push({
            let e = e.clone();
            async move {
                sender
                    .write()
                    .await
                    .1
                    .send(e)
                    .expect("Failed to send message");
            }
        });
    }

    f.collect::<Vec<()>>().await;
}

pub async fn disconnect(
    id: usize,
    user: Arc<User>,
    users: Users,
    subscribed_users: SubscribedUsers,
) {
    info!("Disconnecting {}", id);
    let f = FuturesUnordered::new();
    users.write().await.remove(&id);
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
