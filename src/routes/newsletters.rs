use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;

// handling json data shape
#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

// -- PUBLISH -- //

pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
    let subscribers = get_confirmed_subscribers(&db_pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    // `lazy` difference between it and `context`:
                    // takes closure and closure is ONLY called in case of err
                    // for scenario where context has runtime cost -> avoid 'paying' for err path when op succeeds
                    // -- this specific scenario avoids allocating `format!` call everytime email sent onto the heap (only for failure)
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(err) => {
                tracing::warn!(
                    err.cause_chain = ?err,
                    "Skipping a confirmed subscriber \
                    Their stored contact details are invalid"
                )
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

// // -- HELPERS for PUBLISH -- //
struct ConfirmedSubscriber {
    // email: String,
    email: SubscriberEmail,
}

// adapter between storage and domain layer
#[tracing::instrument(name = "Get confirmed subscribers", skip(db_pool))]
async fn get_confirmed_subscribers(
    db_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(db_pool)
    .await?
    .into_iter()
    .map(|row| match SubscriberEmail::parse(row.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(err) => Err(anyhow::anyhow!(err)),
    })
    .collect();

    // just need `Row` to map data coming out of query -> nested here in fn to comm coupling + keep its use contained to this scope
    // struct Row {
    //     email: String,
    // }

    // let rows = sqlx::query_as!(
    //     // query_as maps retrieved rows to type specified as first arg
    //     Row,
    //     r#"
    //     SELECT email
    //     FROM subscriptions
    //     WHERE status = 'confirmed'
    //     "#,
    // )
    // .fetch_all(db_pool)
    // .await?;

    // let confirmed_subscribers = rows
    //     .into_iter()
    //     // allows us to return iterator containing ONLY items which closure returns `Some`
    //     .filter_map(|row| match SubscriberEmail::parse(row.email) {
    //         Ok(email) => Some(ConfirmedSubscriber { email }),
    //         Err(err) => {
    //             tracing::warn!(
    //                 "A confirmed subscriber is using an invalid email address:\n{}",
    //                 err
    //             );
    //             None
    //         }
    //     })
    //     .collect();

    // let confirmed_subscribers = rows
    //     .into_iter()
    //     .map(|row| match SubscriberEmail::parse(row.email) {
    //         Ok(email) => Ok(ConfirmedSubscriber { email }),
    //         Err(err) => Err(anyhow::anyhow!(err)),
    //     })
    //     .collect();

    Ok(confirmed_subscribers)
}

// -- ERRORS for PUBLISH -- //

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
