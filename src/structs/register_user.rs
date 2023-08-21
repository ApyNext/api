use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegisterUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub birthdate: i64,
    pub biography: String,
    pub is_male: Option<bool>,
}
