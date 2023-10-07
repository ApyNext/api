use std::time::Duration;

use sqlx::PgPool;
use tracing::info;

struct DeleteNotActivatedExpiredAccountsResult {
    total: i64,
}

pub async fn delete_not_activated_expired_accounts(pool: &PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        let count = sqlx::query_as!(DeleteNotActivatedExpiredAccountsResult, r#"WITH updated_rows AS (DELETE FROM users WHERE email_verified = FALSE AND created_at + INTERVAL '10 minutes' < NOW() RETURNING id) SELECT COUNT(id) AS "total!" FROM updated_rows"#).fetch_one(pool).await.unwrap();
        info!("Deleted {} useless account.s", count.total);
    }
}
