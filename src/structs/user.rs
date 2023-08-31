use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct User {
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
}
