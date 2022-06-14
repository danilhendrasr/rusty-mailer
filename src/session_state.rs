use std::future::{ready, Ready};

use actix_session::{Session, SessionExt};
use actix_web::FromRequest;
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub fn renew(&self) {
        self.0.renew()
    }

    pub fn set_user_id(&self, value: Uuid) -> Result<(), serde_json::Error> {
        self.0.insert(Self::USER_ID_KEY, value)
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, serde_json::Error> {
        self.0.get::<Uuid>(Self::USER_ID_KEY)
    }

    pub fn log_out(&self) {
        self.0.purge();
    }
}

impl FromRequest for TypedSession {
    // This is a complicated way of saying
    // "We return the same error returned by the
    // implementation of `FromRequest` for `Session`".
    type Error = <Session as FromRequest>::Error;

    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
