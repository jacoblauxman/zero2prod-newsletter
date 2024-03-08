use crate::session_state::TypedSession;

use actix_web::http::header::ContentType;
use actix_web::http::header::LOCATION;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

// return opaque `500` for user but preserve err root cause (logging)
fn err500<T>(err: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(err)
}

// -- ADMIN DASHBOARD -- //

pub async fn admin_dashboard(
    session: TypedSession,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    // deserialization may fail -> return opaque err above in that case
    // let username = if let Some(user_id) = session.get::<Uuid>("user_id").map_err(err500)? {
    let username = if let Some(user_id) = session.get_user_id().map_err(err500)? {
        get_username(user_id, &db_pool).await.map_err(err500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
        <html lang="en">
        <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin Dashboard</title>
        </head>
        <body>
        <p>
        Welcome {username}!</p>
        </body>
        </html>
        "#
        )))
}

// -- HELPERS for ADMIN DASHBOARD -- //

#[tracing::instrument(name = "Get username", skip(db_pool))]
async fn get_username(user_id: Uuid, db_pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(db_pool)
    .await
    .context("Failed to perform query to retrieve username")?;

    Ok(row.username)
}
