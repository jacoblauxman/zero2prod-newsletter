use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

use unicode_segmentation::UnicodeSegmentation;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(serde::Deserialize)]
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
        // req_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    // retrieving connection from app state
    db_pool: web::Data<PgPool>,
) -> HttpResponse {
    let name = match SubscriberName::parse(form.0.name) {
        Ok(name) => name,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let email = match SubscriberEmail::parse(form.0.email) {
        Ok(email) => email,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let new_subscriber = NewSubscriber { email, name };

    // sqlx may fail in querying so returns `Result` - match statement for err handling variant
    match insert_subscriber(&db_pool, &new_subscriber).await {
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
    skip(new_subscriber, db_pool)
)]
pub async fn insert_subscriber(
    db_pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
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
