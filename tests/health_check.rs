use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;

#[tokio::test]
async fn health_check_works() {
    let app_address = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health-check", app_address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_if_data_valid() {
    let app_address = spawn_app();
    let configuration = get_configuration().expect("Failed to get configurations");
    let connection_string = configuration.database.connection_string();
    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to the database");
    let client = reqwest::Client::new();

    // Simulate data from a url-encoded form
    let body = "name=danil%hendra&email=danilhendrasr%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(200, response.status().as_u16());

    // This might look like an error in VSCode, but it's not
    // You need to set a DATABASE_URL key to rust-analyzer.runnableEnv setting
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&mut connection)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "danilhendrasr@gmail.com");
    assert_eq!(saved.name, "danil hendra");
}

#[tokio::test]
async fn subscribe_returns_400_if_data_invalid() {
    let app_address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=danil%20hendra", "missing email"),
        ("email=danilhendrasr%40gmail.com", "missing name"),
        ("", "missing both name and email"),
    ];

    for (body, error_message) in test_cases {
        let response = client
            .post(format!("{}/subscriptions", app_address))
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

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let our_port = listener.local_addr().unwrap().port();

    let server = zero2prod::run(listener).expect("Failed to bind to address");
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", our_port)
}
