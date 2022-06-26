use actix_web::{http::header, HttpResponse};

pub fn e500<T>(err: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(err)
}

pub fn e400<T>(err: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorBadGateway(err)
}

pub fn see_other(destination: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, destination))
        .finish()
}
