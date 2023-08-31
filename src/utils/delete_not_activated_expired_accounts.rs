use std::{thread, time::Duration};

use shuttle_runtime::tracing::info;
use sqlx::PgPool;

struct DeleteNotActivatedExpiredAccountsResult {
    total: i64,
}

pub async fn delete_not_activated_expired_accounts(pool: &PgPool) {
    loop {
        let count = sqlx::query_as!(DeleteNotActivatedExpiredAccountsResult, r#"WITH updated_rows AS (DELETE FROM users WHERE email_verified = FALSE AND created_at + INTERVAL '10 minutes' < NOW() RETURNING id) SELECT COUNT(id) AS "total!" FROM updated_rows"#).fetch_one(pool).await.unwrap();
        info!("Deleted {} useless account.s", count.total);
        thread::sleep(Duration::from_secs(60));
    }
}
