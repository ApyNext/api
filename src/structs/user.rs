use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct User {
    pub id: u128,
    pub username: String,
    pub email: String,
    pub password: String,
    pub birthdate: DateTime<Utc>,
    pub dark_mode: bool,
    pub biography: String,
}
