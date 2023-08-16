use axum::extract::State;
use sqlx::PgPool;

pub async fn test(State(pool): State<PgPool>) {
    let row: (i128,) = sqlx::query_as("SELECT $1")
        .bind(10)
        .fetch_one(&pool)
        .await
        .unwrap();
}
