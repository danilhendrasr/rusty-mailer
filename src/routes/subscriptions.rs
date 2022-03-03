use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubscriptionData {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(data, db_pool),
    fields(
        subscriber_email = %data.email,
        subscriber_name = %data.name
    )
)]
pub async fn subscribe(
    data: web::Form<SubscriptionData>,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    if is_name_valid(&data.name) {
        return HttpResponse::BadRequest().finish();
    }

    match insert_subscriber(&db_pool, &data).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(name = "Saving new subscriber details to the db", skip(db_pool, data))]
pub async fn insert_subscriber(
    db_pool: &PgPool,
    data: &SubscriptionData,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        data.email,
        data.name,
        Utc::now()
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(())
}

pub fn is_name_valid(name: &str) -> bool {
    let is_empty_or_whitespace = name.trim().is_empty();
    let is_too_long = name.graphemes(true).count() > 256;

    let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
    let contains_forbidden_char = name.chars().any(|c| forbidden_chars.contains(&c));

    !(is_empty_or_whitespace || is_too_long || contains_forbidden_char)
}
