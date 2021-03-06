use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;
use uuid::Uuid;

pub async fn publish_newsletter_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();
    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let idempotency_key = Uuid::new_v4();

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
    <html>
        <head></head>
        <body>
            {}
            <form action="/admin/newsletters" method="POST">
                <input type="hidden" name="idempotency_key" value="{idempotency_key}"/>
                <input type="text" name="title" placeholder="Title"/>
                <textarea name="text_content" placeholder="Text Content"></textarea>
                <textarea name="html_content" placeholder="HTML Content"></textarea>
                <input type="submit" value="Publish"/>
            </form>
        </body>
    </html>"#,
            error_html
        ))
}
