use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DBSettings},
    email_client::EmailClient,
    telemetry::{get_subscriber, init_subscriber},
};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let our_port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", our_port);

    let mut configuration = get_configuration().expect("Failed to get configurations");
    configuration.database.db_name = Uuid::new_v4().to_string();
    let connection_pool = configure_db(&configuration.database).await;

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

    let server = zero2prod::run(listener, connection_pool.clone(), email_client)
        .expect("Failed to bind to address");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool,
    }
}

async fn configure_db(settings: &DBSettings) -> PgPool {
    println!("tayoy {:?}", &settings.connection_string_wo_db());
    let mut connection = PgConnection::connect(&settings.connection_string_wo_db().expose_secret())
        .await
        .expect("Failed connecting to the database");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, settings.db_name).as_str())
        .await
        .expect(format!("Failed to create {} database", settings.db_name).as_str());

    let connection_pool = PgPool::connect(&settings.connection_string().expose_secret())
        .await
        .expect(format!("Failed to connect to database {}", settings.db_name).as_str());

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
