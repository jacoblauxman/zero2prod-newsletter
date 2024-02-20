use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
// temp fix re: warnings of unused vars
#[allow(dead_code)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(
    form: web::Form<FormData>,
    // retrieving connection from app state
    db_pool: web::Data<PgPool>,
) -> HttpResponse {
    // for logging + correlation of info create unique id per request
    let req_id = Uuid::new_v4();
    log::info!(
        "req_id {} - Adding `{}` `{}` as a new subscriber",
        req_id,
        form.email,
        form.name
    );
    log::info!(
        "req_id {} - Saving new subscriber details in the database",
        req_id
    );
    // sqlx may fail in querying so returns `Result` - match statement for err handling variant
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool.get_ref())
    // `get_ref` for immut ref to PgPool wrapped by web::Data
    .await
    {
        Ok(_) => {
            log::info!("req_id {} - New subscriber details saved", req_id);
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            log::error!("req_id {} - Failed to execute query: {:?}", req_id, e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
