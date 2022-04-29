use crate::startup::HmacSecret;
use actix_web::{http::header::ContentType, web, HttpResponse};
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use sha2::Sha256;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    pub fn verify(&self, hmac_secret: &HmacSecret) -> Result<&str, anyhow::Error> {
        let tag = hex::decode(&self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac =
            Hmac::<Sha256>::new_from_slice(hmac_secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(&self.error)
    }
}

pub async fn login_form(
    params: Option<web::Query<QueryParams>>,
    hmac_secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error_message = match params {
        None => "".into(),
        Some(query_param) => match query_param.verify(&hmac_secret) {
            Ok(error_message) => {
                format!(
                    "<p><i>{}</i></p>",
                    htmlescape::encode_minimal(&error_message)
                )
            }
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to verify query parameters using the HMAC tag."
                );

                "".into()
            }
        },
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
