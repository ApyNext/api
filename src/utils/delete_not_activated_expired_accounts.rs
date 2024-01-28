use std::{ops::Sub, time::Duration};

use diesel::prelude::*;
use diesel::{ExpressionMethods, PgConnection, QueryDsl};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::info;

pub async fn delete_not_activated_expired_accounts(mut pool: RwLock<PgConnection>) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        use crate::schema::account::dsl::*;
        let count = diesel::delete(
            account
                .filter(email_verified.eq(false))
                .filter(created_at.lt(OffsetDateTime::now_utc().sub(Duration::from_secs(600)))),
        )
        .execute(pool.get_mut())
        .unwrap();
        info!("Deleted {} useless account.s", count);
    }
}
