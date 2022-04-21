use actix_web::{http::header::ContentType, web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>,
}

pub async fn login_form(params: web::Query<QueryParams>) -> HttpResponse {
    let error_message = match params.0.error {
        None => "".into(),
        Some(error) => format!("<p><i>{}</i></p>", error),
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <meta http-equiv="X-UA-Compatible" content="IE=edge">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>Login</title>
        </head>
        <body>
            {}
          <form action="/login" method="post">
            <input type="text" placeholder="Username" name="username">
            <input type="password" placeholder="Password" name="password">
            <input type="submit" value="Login">
          </form>
        </body>
        </html>"#,
            error_message,
        ))
}
