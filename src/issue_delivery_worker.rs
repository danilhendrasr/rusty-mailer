use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction};
use tracing::{field::display, Span};
use uuid::Uuid;

use crate::{
    configuration::Settings, domains::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool,
};

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty,
    ),
    err
)]
pub async fn try_execute_task(
    email_client: &EmailClient,
    db_pool: &PgPool,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let task = dequeue_task(db_pool).await?;
    if task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }

    let (transaction, issue_id, subscriber_email) = task.unwrap();

    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&subscriber_email));

    match SubscriberEmail::parse(subscriber_email.clone()) {
        Ok(subscriber_email) => {
            let issue = get_issue(issue_id, db_pool).await?;
            if let Err(e) = email_client
                .send_email(
                    &subscriber_email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to a confirmed subscriber. \
                        Skipping."
                )
            }
        }
        Err(error) => {
            tracing::error!(
                error.cause_chain = ?error,
                "Skipping a confirmed subscriber. \
                Their stored email address is invalid."
            )
        }
    }

    delete_task(issue_id, &subscriber_email, transaction).await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db_pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut transaction = db_pool.begin().await?;

    let row = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
    "#
    )
    .fetch_optional(&mut transaction)
    .await?;

    if let Some(row) = row {
        Ok(Some((
            transaction,
            row.newsletter_issue_id,
            row.subscriber_email,
        )))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    issue_id: Uuid,
    subscriber_email: &str,
    mut transaction: PgTransaction,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
            DELETE FROM issue_delivery_queue
            WHERE
                newsletter_issue_id = $1 AND
                subscriber_email = $2
        "#,
        issue_id,
        subscriber_email
    )
    .execute(&mut transaction)
    .await?;

    transaction.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(skip_all)]
async fn get_issue(issue_id: Uuid, db_pool: &PgPool) -> Result<NewsletterIssue, sqlx::Error> {
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE
            id = $1
    "#,
        issue_id
    )
    .fetch_one(db_pool)
    .await?;

    Ok(issue)
}

async fn worker_loop(email_client: EmailClient, db_pool: PgPool) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&email_client, &db_pool).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email_client.client();

    worker_loop(email_client, connection_pool).await
}
