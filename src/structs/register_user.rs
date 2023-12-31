use serde::Deserialize;

#[derive(Deserialize)]
pub struct RegisterUser {
    pub username: String,
    pub email: String,
    pub password: String,
    pub birthdate: i64,
    pub is_male: Option<bool>,
}
