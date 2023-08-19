mod middlewares;
mod routes;
mod structs;
mod utils;

use axum::{middleware, routing::post, Router};

use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use middlewares::logger_middleware::logger_middleware;
use routes::register_route::register_route;
use shuttle_secrets::SecretStore;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pool: PgPool,
    smtp_client: SmtpTransport,
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_secrets::Secrets] secrets: SecretStore,
    #[shuttle_shared_db::Postgres(local_uri = "{secrets.DATABASE_URL}")] pool: PgPool,
) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let smtp_client = SmtpTransport::relay("mail.creativeblogger.org")
        .unwrap()
        .credentials(Credentials::new(
            secrets.get("EMAIL").unwrap(),
            secrets.get("EMAIL_PASSWORD").unwrap(),
        ))
        .build();

    println!("{}", smtp_client.test_connection().unwrap());

    let app_state = AppState { pool, smtp_client };

    let router = Router::new()
        .route("/register", post(register_route))
        .layer(middleware::from_fn(logger_middleware))
        .with_state(app_state);

    Ok(router.into())
}
