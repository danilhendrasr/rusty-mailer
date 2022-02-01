use actix_web::{web, HttpResponse, Responder};

#[derive(serde::Deserialize)]
pub struct SubscriptionData {
    name: String,
    email: String,
}

pub async fn subscriptions(_data: web::Form<SubscriptionData>) -> impl Responder {
    HttpResponse::Ok()
}
