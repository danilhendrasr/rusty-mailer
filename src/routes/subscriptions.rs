use actix_web::{web, HttpResponse, ResponseError, Result};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domains::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(serde::Deserialize)]
pub struct SubscriptionData {
    name: String,
    email: String,
}

impl TryFrom<SubscriptionData> for NewSubscriber {
    type Error = String;

    fn try_from(value: SubscriptionData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { name, email })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(data, db_pool, email_client, base_url),
    fields(
        subscriber_email = %data.email,
        subscriber_name = %data.name
    )
)]
pub async fn subscribe(
    data: web::Form<SubscriptionData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = data.0.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = db_pool.begin().await.map_err(|e| {
        SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to acquire connection from DB pool.".into(),
        )
    })?;

    let new_subscriber_s_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(|e| {
            SubscribeError::UnexpectedError(Box::new(e), "Failed to insert new subscriber.".into())
        })?;

    let subscription_token = generate_subscription_token();
    insert_subscription_token(&mut transaction, new_subscriber_s_id, &subscription_token)
        .await
        .map_err(|e| {
            SubscribeError::UnexpectedError(
                Box::new(e),
                "Failed to insert subscription token.".into(),
            )
        })?;

    transaction.commit().await.map_err(|e| {
        SubscribeError::UnexpectedError(
            Box::new(e),
            "Failed to commit SQL transaction for saving new subscriber.".into(),
        )
    })?;

    send_confirmation_email(
        new_subscriber,
        &email_client,
        &base_url.0,
        &subscription_token,
    )
    .await
    .map_err(|e| {
        SubscribeError::UnexpectedError(Box::new(e), "Failed to send confirmation email.".into())
    })?;

    Ok(HttpResponse::Ok().finish())
}

pub fn generate_subscription_token() -> String {
    let mut rng = thread_rng();

    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect::<String>()
}

#[tracing::instrument(
    name = "Saving new subscription token to subscription_tokens table",
    skip(transaction, subscriber_id, subscription_token)
)]
pub async fn insert_subscription_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), InsertTokenError> {
    sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscriber_id, subscription_token) VALUES ($1, $2)
    "#,
        subscriber_id,
        subscription_token
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        InsertTokenError(e)
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Saving new subscriber details to the db",
    skip(transaction, new_subscriber)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
    "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Send a confirmation email to the new subscriber",
    skip(new_subscriber, email_client)
)]
async fn send_confirmation_email(
    new_subscriber: NewSubscriber,
    email_client: &EmailClient,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
    Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    let text_body = format!(
        "Welcome to our newsletter!\n Visit {} to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &text_body)
        .await
}

pub struct InsertTokenError(sqlx::Error);

impl std::fmt::Debug for InsertTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for InsertTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "A database error has occured while trying to insert new subscription token."
        )
    }
}

impl std::error::Error for InsertTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();

    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error("{1}")]
    UnexpectedError(#[source] Box<dyn std::error::Error>, String),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => reqwest::StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_, _) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
