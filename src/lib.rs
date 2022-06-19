use std::net::TcpListener;

use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_lab::middleware::from_fn;
use authentication::reject_anonymous_users;
use email_client::EmailClient;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use startup::ApplicationBaseUrl;
use tracing_actix_web::TracingLogger;

pub mod authentication;
pub mod configuration;
pub mod domains;
pub mod email_client;
pub mod routes;
pub mod session_state;
pub mod startup;
pub mod telemetry;
pub mod utils;

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let hmac_secret = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(hmac_secret.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                hmac_secret.clone(),
            ))
            .wrap(TracingLogger::default())
            .route("/health-check", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            .route("/subscriptions/confirm", web::get().to(routes::confirm))
            .route("/", web::get().to(routes::home))
            .route("/login", web::get().to(routes::login_form))
            .route("/login", web::post().to(routes::login))
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_users))
                    .route("/dashboard", web::get().to(routes::admin_dashboard))
                    .route(
                        "/newsletters",
                        web::get().to(routes::publish_newsletter_form),
                    )
                    .route("/newsletters", web::post().to(routes::publish_newsletter))
                    .route("/password", web::get().to(routes::change_password_form))
                    .route("/password", web::post().to(routes::change_password))
                    .route("/logout", web::post().to(routes::logout)),
            )
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
