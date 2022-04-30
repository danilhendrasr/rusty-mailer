use crate::helpers::spawn_app;

#[tokio::test]
pub async fn redirect_with_error_message_on_failure() {
    let app = spawn_app().await;

    let body = serde_json::json!({
      "username": "random-username",
      "password": "random-password",
    });

    let response = app.post_login(&body).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");

    // Check if login page contains the error message
    let login_page_html = app.get_login_html().await;
    assert!(login_page_html.contains("<p><i>Invalid credentials.</i></p>"));

    // Reload the login page and check again if it still contains the error message (it shouldn't)
    let login_page_html = app.get_login_html().await;
    assert!(!login_page_html.contains("<p><i>Invalid credentials.</i></p>"));
}
