use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::PgPool;
use zero2prod::configuration::get_configuration;
use zero2prod::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

use zero2prod::email_client::EmailClient;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to get configurations");
    let db_pool = PgPool::connect_lazy(configuration.database.connection_string().expose_secret())
        .expect("Failed to connect to database");

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
    let listener = TcpListener::bind(address).expect("Failed binding to port");

    run(listener, db_pool, email_client)?.await
}
