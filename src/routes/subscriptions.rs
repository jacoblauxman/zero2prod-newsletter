use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
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
    // Spans (sim. to logs) have associated level ie `info_span` is set at info-level
    let req_span = tracing::info_span!(
        "req_id {} - Adding a new subscriber",
        %req_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
        // `%` tells tracing to use `Display` implementations for logging, we can also alias structured info as k/v pairs
    );

    // you have to explicitly step into span via `enter` to activate it
    let _req_span_guard = req_span.enter();
    // returns `Entered` ie a `guard` -- as long as not dropped all downstream spans / log events will be children of the (entered) span

    // Resource Acquisition Is Initialization (RAII)

    // no `enter` for query_span - `instrument` handles it at right moments in query future lifetime
    let query_span = tracing::info_span!("Saving new subscriber details in the database");
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
    .execute(db_pool.get_ref()) // `get_ref` for immut ref to PgPool wrapped by web::Data
    // attach the span `instrumentation` via tracing's Instrument, then await -- future entered every time polled by executor - exited every time parked (on success, give success message)
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            tracing::info!("req_id {} - New subscriber details saved", req_id);
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!("req_id {} - Failed to execute query: {:?}", req_id, e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
