use crate::schema::account;
use diesel::{
    prelude::{Insertable, Queryable},
    sql_types::Timestamptz,
    Selectable,
};
use time::OffsetDateTime;

#[derive(Insertable)]
#[diesel(table_name = account)]
pub struct NewAccount {
    pub username: String,
    pub email: String,
    pub password: String,
    pub birthdate: OffsetDateTime,
    //TODO perhaps add it
    //pub dark_mode: bool,
    //pub biography: String,
    pub token: String,
    pub is_male: Option<bool>,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = account)]
pub struct PublicAccount<'a> {
    pub id: i64,
    pub username: &'a str,
    pub biography: &'a str,
    pub created_at: OffsetDateTime,
    pub permission: u8,
}
