use time::OffsetDateTime;

use super::account::AccountPermission;

pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author: PublicPostAuthor,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(serde::Serialize)]
pub struct PublicPost {
    pub id: i64,
    pub author: PublicPostAuthor,
    pub title: String,
    pub content: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(serde::Serialize)]
pub struct PublicPostAuthor {
    pub id: i64,
    pub username: String,
    pub permission: AccountPermission,
}
