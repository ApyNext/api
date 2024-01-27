use serde::Serialize;
use time::OffsetDateTime;

/*#[derive(Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub author: String,
    pub title: String,
    pub content: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}*/

#[derive(sqlx::FromRow, Serialize)]
pub struct PublicPost {
    pub id: i64,
    pub author: Author,
    pub title: String,
    pub content: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct Author {
    pub id: i64,
    pub username: String,
    pub permission: i64,
}
