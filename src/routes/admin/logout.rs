use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::{session_state::TypedSession, utils::see_other};

pub async fn logout(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    session.log_out();
    FlashMessage::info("You've logged out successfully.").send();
    Ok(see_other("/login"))
}
