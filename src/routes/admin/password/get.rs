use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn change_password_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut error_html = String::new();
    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
    <html>
      <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Change Password</title>
      </head>
      <body>
        {error_html}
        <form action="/admin/password" method="POST">
          <input type="password" placeholder="Enter current password" name="current_password" />
          <input type="password" placeholder="Enter new password" name="new_password" />
          <input type="password" placeholder="Enter new password again" name="new_password_check" />
          <input type="submit" value="Save" />
        </form>
        <a href="/admin/dashboard">&lt; - Back</a>
      </body>
    </html>"#
        )))
}
