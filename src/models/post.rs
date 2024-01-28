use diesel::prelude::Insertable;
use diesel::prelude::Identifiable;
use serde::Deserialize;
use crate::schema::{post::dsl::post, post_author::dsl::post_author};

#[derive(Insertable, Deserialize)]
#[diesel(table_name = post)]
pub struct NewPost {
    title: String,
    content: String,
}

#[derive(Identifiable)]
#[diesel(table_name = post_author)]
pub struct PostAuthor {
    post_id: i64,
    author_id: i64
}
