use serde::Serialize;
use time::OffsetDateTime;

pub struct Account {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password: String,
    pub birthdate: OffsetDateTime,
    pub dark_mode: bool,
    pub biography: String,
    pub token: String,
    pub is_male: Option<bool>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub email_verified: bool,
    pub is_banned: bool,
    pub permission: i64,
}

pub struct PublicAccount {
    pub id: i64,
    pub username: String,
    pub biography: String,
    pub created_at: OffsetDateTime,
    pub permission: i64,
}

// #[derive(sqlx::FromRow, Serialize)]
// pub enum AccountPermission {
//     User = 0,
//     Moderator = 1,
//     Administrator = 2,
// }
