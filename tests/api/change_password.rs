use uuid::Uuid;

use crate::helpers::spawn_app;

#[tokio::test]
async fn must_be_logged_in_to_see_change_password_form() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn must_be_logged_in_to_change_password() {
    let app = spawn_app().await;
    let current_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();

    let body = serde_json::json!({
      "current_password": &current_password,
      "new_password": &new_password,
      "new_password_check": &new_password
    });
    let response = app.post_change_password(&body).await;

    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
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

    let new_password = Uuid::new_v4().to_string();

    let change_password_response = app
        .post_change_password(&serde_json::json!({
          "current_password": &app.test_user.password,
          "new_password": &new_password,
          "new_password_check": "Heytayo"
        }))
        .await;
    assert_eq!(change_password_response.status().as_u16(), 303);
    assert_eq!(
        change_password_response.headers().get("Location").unwrap(),
        "/admin/password"
    );

    let change_password_form = app.get_change_password_html().await;
    assert!(change_password_form.contains("New password must match"));
}

#[tokio::test]
async fn current_password_must_be_correct() {
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

    let new_password = Uuid::new_v4().to_string();

    let change_password_response = app
        .post_change_password(&serde_json::json!({
          "current_password": "Heytayo",
          "new_password": &new_password,
          "new_password_check": &new_password
        }))
        .await;
    assert_eq!(change_password_response.status().as_u16(), 303);
    assert_eq!(
        change_password_response.headers().get("Location").unwrap(),
        "/admin/password"
    );

    let change_password_form = app.get_change_password_html().await;
    assert!(change_password_form.contains("Current password is incorrect."));
}

#[tokio::test]
async fn new_password_must_be_between_12_and_128_characters_long() {
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

    let new_password = "Pass";

    let change_password_response = app
        .post_change_password(&serde_json::json!({
          "current_password": &app.test_user.password,
          "new_password": &new_password,
          "new_password_check": &new_password
        }))
        .await;
    assert_eq!(change_password_response.status().as_u16(), 303);
    assert_eq!(
        change_password_response.headers().get("Location").unwrap(),
        "/admin/password"
    );

    let change_password_form = app.get_change_password_html().await;
    assert!(
        change_password_form.contains("New password must be between 12 and 128 characters long.")
    );
}

#[tokio::test]
async fn changing_password_works() {
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

    let new_password = Uuid::new_v4().to_string();

    let change_password_response = app
        .post_change_password(&serde_json::json!({
          "current_password": &app.test_user.password,
          "new_password": &new_password,
          "new_password_check": &new_password
        }))
        .await;
    assert_eq!(change_password_response.status().as_u16(), 303);
    assert_eq!(
        change_password_response.headers().get("Location").unwrap(),
        "/admin/password"
    );
    println!("{}", change_password_response.text().await.unwrap());

    let change_password_html = app.get_change_password_html().await;
    assert!(change_password_html.contains("Password changed successfully."));

    let logout_response = app.post_logout().await;
    assert_eq!(logout_response.status().as_u16(), 303);
    assert_eq!(logout_response.headers().get("Location").unwrap(), "/login");

    let login_html = app.get_login_html().await;
    assert!(login_html.contains("You've logged out successfully."));

    let login_response = app
        .post_login(&serde_json::json!({
            "username": &app.test_user.username,
            "password": &new_password
        }))
        .await;
    assert_eq!(login_response.status().as_u16(), 303);
    assert_eq!(
        login_response.headers().get("Location").unwrap(),
        "/admin/dashboard"
    );
}
