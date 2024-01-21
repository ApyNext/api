use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize)]
pub struct Post {
    id: i64,
    author: i64,
    title: String,
    content: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}
