use actix_web::{http::header, web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{session_state::TypedSession, utils::e500};

pub async fn admin_dashboard(
    session: TypedSession,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = if let Some(user_id) = session.get_user_id().map_err(e500)? {
        get_username(user_id, &db_pool).await.map_err(e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/login"))
            .finish());
    };

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
        user_id
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
