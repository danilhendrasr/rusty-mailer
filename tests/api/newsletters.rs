use std::time::Duration;

use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{spawn_app, ConfirmationLink, TestApp};

#[tokio::test]
async fn must_be_logged_in_to_see_newsletter_issue_form() {
    let app = spawn_app().await;

    let response = app.get_publish_newsletter().await;
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn must_be_logged_in_post_new_newsletter_issue() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "title": "Newsletter Title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as plain text</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });

    let response = app.post_publish_newsletter(&body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    let login_response = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_eq!(login_response.status().as_u16(), 303);
    assert_eq!(
        login_response.headers().get("Location").unwrap(),
        "/admin/dashboard"
    );

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter Title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as plain text</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/newsletters"
    );

    let publish_newsletter_html = app.get_publish_newsletter_html().await;
    assert!(publish_newsletter_html.contains("Success publishing new newsletter issue."));
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    let login_response = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_eq!(login_response.status().as_u16(), 303);
    assert_eq!(
        login_response.headers().get("Location").unwrap(),
        "/admin/dashboard"
    );

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter Title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as plain text</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });

    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/newsletters"
    );

    let publish_newsletter_html = app.get_publish_newsletter_html().await;
    assert!(publish_newsletter_html.contains("Success publishing new newsletter issue."));
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter Title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as plain text</p>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/newsletters"
    );

    let publish_newsletter_html = app.get_publish_newsletter_html().await;
    assert!(publish_newsletter_html.contains("Success publishing new newsletter issue."));

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/newsletters"
    );

    let publish_newsletter_html = app.get_publish_newsletter_html().await;
    assert!(publish_newsletter_html.contains("Success publishing new newsletter issue."));
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let request_body = serde_json::json!({
        "title": "Newsletter Title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as plain text</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });

    let response1 = app.post_publish_newsletter(&request_body);
    let response2 = app.post_publish_newsletter(&request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
}

#[tokio::test]
async fn returns_400_given_invalid_body() {
    let app = spawn_app().await;
    let idempotency_key = Uuid::new_v4().to_string();
    let test_cases = vec![
        (
            serde_json::json!({
                "text_content": "Newsletter body as plain text",
                "html_content": "<p>Newsletter body as plain text</p>",
                "idempotency_key": idempotency_key
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "idempotency_key": idempotency_key
            }),
            "missing content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter title",
                "content": "Newsletter body as plain text",
                "idempotency_key": idempotency_key
            }),
            "malformed content",
        ),
    ];

    let login_response = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &app.test_user.password,
        }))
        .await;
    assert_eq!(login_response.status().as_u16(), 303);
    assert_eq!(
        login_response.headers().get("Location").unwrap(),
        "/admin/dashboard"
    );

    for (body, state) in test_cases {
        let response = app.post_publish_newsletter(&body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {}",
            state
        );
    }
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLink {
    let body = "name=danil%20hendra&email=danilhendrasr%40gmail.com";
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscription(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_link_from_email_body(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;

    reqwest::get(confirmation_link.plain_text)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
