use crate::helpers::spawn_app;

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
