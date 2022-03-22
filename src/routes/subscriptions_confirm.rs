use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct QueryParam {
    pub subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(param))]
pub async fn confirm(param: web::Query<QueryParam>, db_pool: web::Data<PgPool>) -> HttpResponse {
    let subscriber_id =
        match get_subscriber_id_from_token(&db_pool, &param.subscription_token).await {
            Ok(value) => value,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    match subscriber_id {
        None => HttpResponse::Unauthorized().finish(),
        Some(id) => match confirm_user_subscription(&db_pool, id).await {
            Err(_) => HttpResponse::InternalServerError().finish(),
            Ok(_) => HttpResponse::Ok().finish(),
        },
    }
}

#[tracing::instrument(name = "Confirm user subscription", skip(db_pool, subscriber_id))]
async fn confirm_user_subscription(
    db_pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Get subscriber ID from subscription token",
    skip(db_pool, subscription_token)
)]
async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}
