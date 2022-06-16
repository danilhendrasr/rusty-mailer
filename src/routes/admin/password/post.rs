use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::{
    authentication::{self, validate_credentials, AuthError, UserCredentials, UserId},
    routes::admin::dashboard::get_username,
    utils::{e500, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    pub current_password: Secret<String>,
    pub new_password: Secret<String>,
    pub new_password_check: Secret<String>,
}

pub async fn change_password(
    form_data: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    if form_data.new_password.expose_secret() != form_data.new_password_check.expose_secret() {
        FlashMessage::error("New password must match").send();
        return Ok(see_other("/admin/password"));
    }

    let password_length = form_data.new_password.expose_secret().chars().count();
    if !(12..=128).contains(&password_length) {
        FlashMessage::error("New password must be between 12 and 128 characters long.").send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(*user_id, &db_pool).await.map_err(e500)?;

    let user_credentials = UserCredentials {
        username,
        password: form_data.0.current_password,
    };

    if let Err(e) = validate_credentials(user_credentials, &db_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("Current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    authentication::change_password(*user_id, form_data.0.new_password, &db_pool)
        .await
        .map_err(e500)?;

    FlashMessage::info("Password changed successfully.").send();
    Ok(see_other("/admin/password"))
}
