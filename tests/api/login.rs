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

#[tokio::test]
pub async fn redirect_to_admin_dashboard_on_login_success() {
    let app = spawn_app().await;

    // Login
    let body = serde_json::json!({
      "username": &app.test_user.username,
      "password": &app.test_user.password,
    });
    let response = app.post_login(&body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/dashboard"
    );

    // Assert admin dashboard HTML
    let admin_dashboard_html = app.get_admin_dashboard_html().await;
    assert!(admin_dashboard_html.contains(&format!("Welcome {}", app.test_user.username)));
}

#[tokio::test]
pub async fn must_login_to_access_admin_dashboard() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}
