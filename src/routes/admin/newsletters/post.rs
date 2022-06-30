use actix_web::{http, http::header, http::header::HeaderValue, web, HttpResponse, ResponseError};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    authentication::UserId,
    domains::{save_response, try_processing, IdempotencyKey, NextAction},
    routes::error_chain_fmt,
    utils::{e400, e500, see_other},
};

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed.")]
    _AuthError(#[source] anyhow::Error),
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
            PublishError::_AuthError(_) => {
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
pub struct FormData {
    pub title: String,
    pub text_content: String,
    pub html_content: String,
    pub idempotency_key: String,
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(
    "Publishing newsletter",
    skip(form_data, db_pool),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    form_data: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> actix_web::Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form_data.0;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;
    let mut transaction = match try_processing(*user_id, &idempotency_key, &db_pool)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(
        NewsletterIssue {
            title,
            text_content,
            html_content,
        },
        &mut transaction,
    )
    .await
    .context("Failed to store newsletter issue details.")
    .map_err(e500)?;

    enqueue_issue_delivery(issue_id, &mut transaction)
        .await
        .context("Failed enqueueing issue delivery task.")
        .map_err(e500)?;

    let response = see_other("/admin/newsletters");
    let response = save_response(*user_id, &idempotency_key, response, transaction)
        .await
        .map_err(e500)?;

    success_message().send();
    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    newsletter_issue: NewsletterIssue,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            id,
            title,
            text_content,
            html_content,
            published_at
        ) VALUES ($1, $2, $3, $4, now())
    "#,
        newsletter_issue_id,
        newsletter_issue.title,
        newsletter_issue.text_content,
        newsletter_issue.html_content
    )
    .execute(transaction)
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_issue_delivery(
    newsletter_issue_id: Uuid,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        ) SELECT $1, email
            FROM subscriptions
            WHERE status = 'confirmed'
    "#,
        newsletter_issue_id
    )
    .execute(transaction)
    .await?;

    Ok(())
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been accepted, emails will go out shortly.")
}
