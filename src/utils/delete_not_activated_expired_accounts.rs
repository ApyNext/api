use std::{thread, time::Duration};

use shuttle_runtime::tracing::info;
use sqlx::PgPool;

pub async fn delete_not_activated_expired_accounts(pool: &PgPool) {
    loop {
        sqlx::query!("DELETE FROM users WHERE email_verified = FALSE AND created_at + INTERVAL '10 minutes' < NOW()").execute(pool).await.unwrap();
        info!("Deleted useless accounts");
        thread::sleep(Duration::from_secs(60));
    }
}
