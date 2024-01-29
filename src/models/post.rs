use time::OffsetDateTime;

use super::account::AccountPermission;
use serde::Serialize;

pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author: PublicPostAuthor,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct PublicPost {
    pub id: i64,
    pub author: PublicPostAuthor,
    pub title: String,
    pub content: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct NotificationPost {
    pub id: i64,
    pub author: PublicPostAuthor,
    pub title: String,
    pub created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct PublicPostAuthor {
    pub id: i64,
    pub username: String,
    pub permission: AccountPermission,
}
