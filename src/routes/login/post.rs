use actix_web::{http::header, web, HttpResponse};
use secrecy::Secret;
use sqlx::PgPool;

use crate::{
    authentication::{validate_credentials, AuthError, UserCredentials},
    routes::error_chain_fmt,
};

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Invalid credentialss.")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong.")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for LoginError {
    fn status_code(&self) -> reqwest::StatusCode {
        // match self {
        //     LoginError::AuthError(_) => reqwest::StatusCode::UNAUTHORIZED,
        //     LoginError::UnexpectedError(_) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        // }
        reqwest::StatusCode::SEE_OTHER
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let urlencoded_error = urlencoding::Encoded(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((
                header::LOCATION,
                format!("/login?error={}", urlencoded_error),
            ))
            .finish()
    }
}

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
) -> Result<HttpResponse, LoginError> {
    let user_credentials = UserCredentials {
        username: form_data.0.username,
        password: Secret::new(form_data.0.password),
    };

    tracing::Span::current().record(
        "username",
        &tracing::field::display(&user_credentials.username),
    );

    let user_id = validate_credentials(user_credentials, &db_pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(user_id));

    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/"))
        .finish())
}
