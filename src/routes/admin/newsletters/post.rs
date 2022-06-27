use actix_web::{http, http::header, http::header::HeaderValue, web, HttpResponse, ResponseError};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domains::{save_response, try_processing, IdempotencyKey, NextAction, SubscriberEmail},
    email_client::EmailClient,
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

#[tracing::instrument(
    "Publishing newsletter",
    skip(form_data, db_pool, email_client),
    fields(user_id=%*user_id)
)]
pub async fn publish_newsletter(
    form_data: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
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
    let transaction = match try_processing(*user_id, &idempotency_key, &db_pool)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(transaction) => transaction,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message().send();
            return Ok(saved_response);
        }
    };

    let confirmed_subscribers = get_confirmed_subscriber(&db_pool).await.map_err(e500)?;
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(confirmed_subscriber) => {
                email_client
                    .send_email(
                        &confirmed_subscriber.email,
                        &title,
                        &html_content,
                        &text_content,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "Failed sending newsletter to {}",
                            &confirmed_subscriber.email
                        )
                    })
                    .map_err(e500)?;
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

    success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(*user_id, &idempotency_key, response, transaction)
        .await
        .map_err(e500)?;

    Ok(response)
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

fn success_message() -> FlashMessage {
    FlashMessage::info("Success publishing new newsletter issue.")
}
