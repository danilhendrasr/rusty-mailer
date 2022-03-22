use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_subscription_token_are_rejected_with_400() {
    let app = spawn_app().await;

    let response = reqwest::get(format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_200() {
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

    let response = reqwest::get(confirmation_link.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_confirmation_link_confirms_the_subscription() {
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

    reqwest::get(confirmation_link.html).await.unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "danilhendrasr@gmail.com");
    assert_eq!(saved.name, "danil hendra");
    assert_eq!(saved.status, "confirmed");
}
