use actix_web::{cookie::Cookie, http::header::ContentType, HttpRequest, HttpResponse};

pub async fn login_form(request: HttpRequest) -> HttpResponse {
    let error_message = match request.cookie("_flash") {
        None => "".into(),
        Some(cookie) => format!("<p><i>{}</i></p>", cookie.value()),
    };

    let mut response = HttpResponse::Ok()
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
        ));

    response
        .add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();

    response
}
