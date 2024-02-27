use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
// use tracing::Instrument;
use uuid::Uuid;

// use unicode_segmentation::UnicodeSegmentation;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(val: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(val.name)?;
        let email = SubscriberEmail::parse(val.email)?;

        Ok(Self { email, name })
    }
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
    // let new_subscriber = match parse_subscriber(form.0) {
    let new_subscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

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
        INSERT INTO subscriptions (id, email, name, subscribed_at,  status)
        VALUES ($1, $2, $3, $4, 'confirmed')
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
