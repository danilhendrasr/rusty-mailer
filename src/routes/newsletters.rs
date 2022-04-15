use actix_web::{
    http,
    http::header,
    http::header::{HeaderMap, HeaderValue},
    web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    domains::SubscriberEmail, email_client::EmailClient, telemetry::spawn_blocking_with_tracing,
};

use super::error_chain_fmt;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(http::StatusCode::UNAUTHORIZED);

                let header_value_authenticate =
                    HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value_authenticate);

                response
            }
        }
    }
}

#[derive(serde::Deserialize)]
pub struct BodyData {
    pub title: String,
    pub content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    pub text: String,
    pub html: String,
}

#[tracing::instrument(
    "Publishing newsletter", 
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse, PublishError> {
    let confirmed_subscribers = get_confirmed_subscriber(&pool).await?;
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(confirmed_subscriber) => {
                email_client
                    .send_email(
                        &confirmed_subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "Failed sending newsletter to {}",
                            &confirmed_subscriber.email
                        )
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored email address is invalid."
                )
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

struct ConfirmedSubscriber {
    pub email: SubscriberEmail,
}

#[tracing::instrument(name = "Getting confirmed subscribers", skip(pool))]
async fn get_confirmed_subscriber(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!("SELECT email FROM subscriptions WHERE status = 'confirmed'")
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(rows)
}

struct UserCredentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Decoding Authorization header.", skip(headers))]
fn basic_authentication(headers: &HeaderMap) -> Result<UserCredentials, anyhow::Error> {
    let auth_header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing.")?
        .to_str()
        .context("The 'Authorization' header is not a valid UTF8 string.")?;

    let auth_base64_str = auth_header_value
        .strip_prefix("Basic ")
        .context("The Authorization scheme is not 'Basic '.")?;

    let decoded_base64_bytes = base64::decode(auth_base64_str)
        .context("Failed decoding the Authorization base64 string.")?;

    let decoded_string = String::from_utf8(decoded_base64_bytes)
        .context("The credentials is not a valid UTF8 string.")?;

    let mut splitted_string = decoded_string.splitn(2, ':');
    let username = splitted_string
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing username in the authentication."))?
        .to_string();

    let password = splitted_string
        .next()
        .ok_or_else(|| anyhow::anyhow!("Missing password in the authentication."))?
        .to_string();

    Ok(UserCredentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(name = "Validating credentials.", skip(credentials, pool))]
async fn validate_credentials(
    credentials: UserCredentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let (user_id, expected_password_hash) =
        get_stored_user_credentials(&credentials.username, pool)
            .await
            .map_err(PublishError::UnexpectedError)?
            .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Username not found.")))?;

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(PublishError::UnexpectedError)??;

    Ok(user_id)
}

#[tracing::instrument(
    name = "Verify password hash.",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), PublishError> {
    let password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed parsing password hash from PHC string.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &password_hash,
        )
        .context("Invalid password.")
        .map_err(PublishError::AuthError)
}

#[tracing::instrument(name = "Get stored user credentials.", skip(username, pool))]
async fn get_stored_user_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT id, password_hash FROM users WHERE username = $1"#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed fetching stored user credentials from the database.")?
    .map(|row| (row.id, Secret::new(row.password_hash)));

    Ok(row)
}
