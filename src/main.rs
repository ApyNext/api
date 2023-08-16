mod routes;

use axum::{routing::get, Router};
use routes::test::test;
use sqlx::PgPool;

async fn hello_world() -> &'static str {
    "Hello, world!"
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://postgres:{secrets.PASSWORD}@localhost:5432/postgres"
    )]
    pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    let router = Router::new().route("/", get(test)).with_state(pool);

    Ok(router.into())
}
