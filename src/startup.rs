use std::net::TcpListener;

use actix_web::dev::Server;
use secrecy::Secret;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

use crate::{
    configuration::{DBSettings, Settings},
    run,
};
use std::time::Duration;

pub fn get_connection_pool(configuration: &DBSettings) -> Pool<Postgres> {
    PgPoolOptions::new()
        .connect_timeout(Duration::from_secs(10))
        .connect_lazy_with(configuration.with_db())
}

pub struct HmacSecret(pub Secret<String>);

// We need to define a wrapper type in order to retrieve the URL
// in the `subscribe` handler.
// Retrieval from the context, in actix-web, is type-based: using
// a raw `String` would expose us to conflicts.
pub struct ApplicationBaseUrl(pub String);

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let db_pool = get_connection_pool(&configuration.database);

        let email_client = configuration.email_client.client();
        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            db_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
