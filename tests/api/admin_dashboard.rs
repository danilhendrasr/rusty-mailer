use crate::helpers::spawn_app;

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

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

    let admin_dashboard_html = app.get_admin_dashboard_html().await;
    assert!(admin_dashboard_html.contains(&format!("Welcome {}", &app.test_user.username)));

    let logout_response = app.post_logout().await;
    assert_eq!(logout_response.status().as_u16(), 303);
    assert_eq!(logout_response.headers().get("Location").unwrap(), "/login");

    let login_html = app.get_login_html().await;
    assert!(login_html.contains("You've logged out successfully."));

    let admin_dashboard_response = app.get_admin_dashboard().await;
    assert_eq!(admin_dashboard_response.status().as_u16(), 303);
    assert_eq!(
        admin_dashboard_response.headers().get("Location").unwrap(),
        "/login"
    );
}
