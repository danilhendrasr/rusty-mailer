use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{authentication::UserId, utils::e500};

pub async fn admin_dashboard(
    user_id: web::ReqData<UserId>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &db_pool).await.map_err(e500)?;

    Ok(HttpResponse::Ok().body(format!(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin Dashboard</title>
            </head>
            <body>
                <p>Welcome {}</p>
                <ol>
                    Available Actions:
                    <li><a href="/admin/newsletters">Send a newsletter issue</a></li>
                    <li><a href="/admin/password">Change password</a></li>
                    <li>
                    <form name="logout_form" action="/admin/logout" method="POST">
                    <input type="submit" value="Logout"/>
                    </form>
                    </li>
                </ol>
            </body>
        <html>
    "#,
        username
    )))
}

#[tracing::instrument(name = "Get username from user ID", skip(db_pool))]
pub async fn get_username(user_id: Uuid, db_pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!("SELECT username FROM users WHERE id = $1", user_id)
        .fetch_one(db_pool)
        .await
        .context("Failed to fetch username from user id.")?;

    Ok(row.username)
}
