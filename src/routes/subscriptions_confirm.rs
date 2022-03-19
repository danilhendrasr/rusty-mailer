use actix_web::{web, HttpResponse};

#[derive(Debug, serde::Deserialize)]
pub struct QueryParam {
    pub subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscription", skip(_param))]
pub async fn confirm(_param: web::Query<QueryParam>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
