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
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| -> String {
        let links = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect::<Vec<linkify::Link>>();

        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let html_link = get_link(&body["html_body"].as_str().unwrap());
    let text_link = get_link(&body["text_body"].as_str().unwrap());

    assert_eq!(html_link, text_link);
}
