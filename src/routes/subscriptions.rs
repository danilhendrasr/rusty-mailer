use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sqlx::PgPool;
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
    let execute_query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at)
    VALUES ($1, $2, $3, $4)
    "#,
        Uuid::new_v4(),
        data.email,
        data.name,
        Utc::now()
    )
    .execute(db_pool.get_ref());

    match execute_query.await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => {
            println!("Failed to execute query");
            HttpResponse::InternalServerError().finish()
        }
    }
}
