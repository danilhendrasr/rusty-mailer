use crate::helpers::spawn_app;

#[tokio::test]
pub async fn respond_with_303_on_failure() {
    let app = spawn_app().await;

    let body = serde_json::json!({
      "username": "random-username",
      "password": "random-password",
    });

    let response = app.post_login(&body).await;
    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();

    assert_eq!(flash_cookie.value(), "Invalid credentials.");
    assert_eq!(response.status().as_u16(), 303);

    // Check if login page contains the error message
    let login_page_html = app.get_login_html().await;
    assert!(login_page_html.contains("<p><i>Invalid credentials.</i></p>"));

    // Reload the login page and check again if it still contains the error message
    let login_page_html = app.get_login_html().await;
    assert!(!login_page_html.contains("<p><i>Invalid credentials.</i></p>"));
}
