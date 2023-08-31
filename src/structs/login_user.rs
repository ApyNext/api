use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginUser {
    pub username_or_email: String,
    pub password: String,
}
