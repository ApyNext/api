use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct User {
    pub id: u128,
    pub username: String,
    pub email: String,
    pub password: String,
    pub birthdate: OffsetDateTime,
    pub dark_mode: bool,
    pub biography: String,
}
