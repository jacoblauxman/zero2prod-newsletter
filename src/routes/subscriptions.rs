use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
// temp fix re: warnings of unused vars
pub struct FormData {
    email: String,
    name: String,
}

// form submission handling / orchestration - invokes insert_subscriber
#[tracing::instrument(
    name = "Adding a new subscriber",
    // tracing by default captures all args to fn, skip used to omit info in log
    skip(form, db_pool),
    fields(
        req_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    // retrieving connection from app state
    db_pool: web::Data<PgPool>,
) -> HttpResponse {
    // sqlx may fail in querying so returns `Result` - match statement for err handling variant
    match insert_subscriber(&db_pool, &form).await {
        Ok(_) => {
            // tracing::info!("req_id {} - New subscriber details saved", req_id);
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}


// inserting subscriber to database
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, db_pool)
)]
pub async fn insert_subscriber(db_pool: &PgPool, form: &FormData) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    // TODO: Better error handling!

    Ok(())
}
