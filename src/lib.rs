use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .route("/health-check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscriptions))
    })
    .listen(listener)?
    .run();

    Ok(server)
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok()
}

#[derive(serde::Deserialize)]
struct SubscriptionData {
    name: String,
    email: String,
}

async fn subscriptions(_data: web::Form<SubscriptionData>) -> impl Responder {
    HttpResponse::Ok()
}
