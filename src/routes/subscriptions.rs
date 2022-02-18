use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct SubscriptionData {
    name: String,
    email: String,
}

pub async fn subscribe(
    data: web::Form<SubscriptionData>,
    db_pool: web::Data<PgPool>,
) -> impl Responder {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!("Adding a new subscriber", %request_id, subscriber_email = %data.email, subscriber_name = %data.name);

    let _request_span_guard = request_span.enter();
    let query_span = tracing::info_span!("Saving new subscriber details to the database");
    let execute_query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        data.email,
        data.name,
        Utc::now()
    );

    match execute_query
        .execute(db_pool.get_ref())
        .instrument(query_span)
        .await
    {
        Ok(_) => {
            tracing::info!("[Request ID: {}] -- New subscriber saved", request_id);
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!(
                "[Request ID: {}] -- Failed to execute query {:?}",
                request_id,
                e
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}
