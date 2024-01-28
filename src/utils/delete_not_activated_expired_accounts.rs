use std::time::Duration;

use tracing::info;

use crate::AppState;

struct Count {
    total: i64,
}

pub async fn delete_not_activated_expired_accounts(app_state: &AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(86400));
    loop {
        interval.tick().await;
        let count = sqlx::query_as!(Count, r#"WITH updated_rows AS (DELETE FROM account WHERE email_verified = FALSE AND created_at + INTERVAL '10 minutes' < NOW() RETURNING id) SELECT COUNT(id) AS "total!" FROM updated_rows"#).fetch_one(&app_state.pool).await.unwrap();
        info!("Deleted {} useless account.s", count.total);
    }
}
