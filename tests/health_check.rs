use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DBSettings};
use zero2prod::email_client::EmailClient;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health-check", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_if_data_valid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Simulate data from a url-encoded form
    let body = "name=danil%20hendra&email=danilhendrasr%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    // This might look like an error in VSCode, but it's not
    // You need to set a DATABASE_URL key to rust-analyzer.runnableEnv setting
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "danilhendrasr@gmail.com");
    assert_eq!(saved.name, "danil hendra");
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=danilhendrasr%40gmail.com", "empty name"),
        ("name=danil%20hendra&email=", "empty email"),
        ("name=danil%20hendra&email=invalid-email", "invalid email"),
    ];

    for (body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_400_if_data_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=danil%20hendra", "missing email"),
        ("email=danilhendrasr%40gmail.com", "missing name"),
        ("", "missing both name and email"),
    ];

    for (body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
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

pub struct TestApp {
    address: String,
    db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let our_port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", our_port);

    let mut configuration = get_configuration().expect("Failed to get configurations");
    configuration.database.db_name = Uuid::new_v4().to_string();
    let connection_pool = configure_db(&configuration.database).await;

    // Setup email client, we're using singleton to utilize reqwest's HTTP connection pooling
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
    );

    let server = zero2prod::run(listener, connection_pool.clone(), email_client)
        .expect("Failed to bind to address");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_db(settings: &DBSettings) -> PgPool {
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
