use std::net::TcpListener;

use actix_web::dev::Server;
use secrecy::ExposeSecret;
use sqlx::{PgPool, Pool, Postgres};

use crate::{configuration::Settings, email_client::EmailClient, run};

pub fn get_connection_pool(connection_string: &str) -> Pool<Postgres> {
    PgPool::connect_lazy(connection_string).expect("Failed to connect to database")
}

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let db_pool =
            get_connection_pool(configuration.database.connection_string().expose_secret());

        // Setup email client, we're using singleton to utilize reqwest's HTTP connection pooling
        let email_client_sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address for email client.");

        let email_client_timeout_duration = configuration.email_client.timeout();

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            email_client_sender_email,
            configuration.email_client.authorization_token,
            email_client_timeout_duration,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );

        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, db_pool, email_client)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
