mod routes;
mod structs;

use axum::{routing::get, Router};
use routes::register_route::register_route;
use sqlx::PgPool;

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:{secrets.PASSWORD}@localhost:5432/postgres"
    )]
    pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let router = Router::new()
        .route("/", get(register_route))
        .with_state(pool);

    Ok(router.into())
}
