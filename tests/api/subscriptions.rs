use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_if_data_valid() {
    let app = spawn_app().await;
    let body = "name=danil%20hendra&email=danilhendrasr%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_subscription(body.into()).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    let body = "name=danil%20hendra&email=danilhendrasr%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscription(body.into()).await;

    // This might look like an error in VSCode, but it's not
    // You need to set a DATABASE_URL key to rust-analyzer.runnableEnv setting
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "danilhendrasr@gmail.com");
    assert_eq!(saved.name, "danil hendra");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=danilhendrasr%40gmail.com", "empty name"),
        ("name=danil%20hendra&email=", "empty email"),
        ("name=danil%20hendra&email=invalid-email", "invalid email"),
    ];

    for (body, error_message) in test_cases {
        let response = app.post_subscription(body.into()).await;

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
    let test_cases = vec![
        ("name=danil%20hendra", "missing email"),
        ("email=danilhendrasr%40gmail.com", "missing name"),
        ("", "missing both name and email"),
    ];

    for (body, error_message) in test_cases {
        let response = app.post_subscription(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let body = "name=danil%20hendra&email=danilhendrasr%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscription(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link_in_it() {
    let app = spawn_app().await;
    let body = "name=danil%20hendra&email=danilhendrasr%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscription(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_link = app.get_confirmation_link_from_email_body(email_request);

    assert_eq!(confirmation_link.html, confirmation_link.plain_text);
}
