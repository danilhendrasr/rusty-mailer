use actix_web::{cookie::Cookie, error::InternalError, http::header, web, HttpResponse};
use secrecy::Secret;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, UserCredentials},
    routes::error_chain_fmt,
};

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Invalid credentials.")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong.")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for LoginError {}

#[derive(serde::Deserialize)]
pub struct LoginFormData {
    pub username: String,
    pub password: String,
}

#[tracing::instrument(
    skip(form_data, db_pool),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    form_data: web::Form<LoginFormData>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let user_credentials = UserCredentials {
        username: form_data.0.username,
        password: Secret::new(form_data.0.password),
    };

    tracing::Span::current().record(
        "username",
        &tracing::field::display(&user_credentials.username),
    );

    match validate_credentials(user_credentials, &db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(user_id));

            Ok(HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };

            let response = HttpResponse::SeeOther()
                .insert_header((header::LOCATION, format!("/login")))
                .cookie(Cookie::new("_flash", e.to_string()))
                .finish();

            Err(InternalError::from_response(e, response))
        }
    }
}
