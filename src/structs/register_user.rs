use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct RegisterUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub birthdate: OffsetDateTime,
    pub biography: String,
}
