use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

// form handling / implementation
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

// -- -- SUBSCRIBE -- -- //

// submission handling / orchestration
#[tracing::instrument(
    name = "Adding a new subscriber",
    // tracing by default captures all args to fn, skip used to omit info in log
    skip(form, db_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    // retrieving connection from app state
    db_pool: web::Data<PgPool>,
    // retrieve email client from app state
    email_client: web::Data<EmailClient>,
    // app env base'd
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    // get subscriber data from form input
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;
    // note: no longer have #[from] for `SubscribeError::ValidationError` - have to map explicitly because `String` doesn't impl Error trait and can't be returned in Error::source (used `None` for error case handling prior)
    // `begin` acquires connection from the db's pool to kick off transaction -- provides way to convert multi-steps of db interaction into 'all-or-nothing'
    let mut transaction = db_pool
        .begin()
        .await
        .context("Failed to acquire Postgres connection from the db pool")?;
    // get subscriber's id from inserting into db, return `500` if fails
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber into the database")?;
    // rand gen'd confirmation token
    let subscription_token = generate_subscription_token();
    // store subscriber's token in db associated to sub's id, `500` if fails
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store confirmation token for new subscriber in db")?;
    // finalize db transaction/commit and return conn to db pool, `500` if fails
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store new subscriber into db")?;
    // send email via external API service, `500` if fails
    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email to new subscriber")?;

    Ok(HttpResponse::Ok().finish())
}

// -- ERRORS for SUBSCRIBE -- //

#[derive(thiserror::Error)]
pub enum SubscribeError {
    // destructuring of fields (0th) within given Error tuple variant
    #[error("{0}")]
    ValidationError(String),
    // `transparent` delegates both `Display` + `source` impl's to type within `UnexpectedError`
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    // we utilize `context` method from `anyhow` to:
    // 1) convert err returned by methods into anyhow::Error
    // 2) enrich with additional context around intention (ie our message)
    // `context` provided via `Context` trait - `anyhow` implements for `Result` (ie an EXTENSION TRAIT)
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// -- -- HELPER LOGIC for SUBSCRIBE -- -- //

// SEND EMAIL confirmation to new subscriber
#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let text_content = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_content = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(
            &new_subscriber.email,
            "Welcome!",
            &html_content,
            &text_content,
        )
        .await
}

// INSERT SUBSCRIBER into database
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );

    transaction.execute(query).await?;
    // note: since propogating err upstream via '?' operator DON'T `tracing::error!` log here!

    Ok(subscriber_id)
}

// INSERT TOKEN into database
#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(|err| {
        // note: since propogating err upstream via '?' operator DON'T `tracing::error!` log here!
        // wrap underlying error
        StoreTokenError(err)
    })?;

    Ok(())
}

// gen random 25 char token for email confirmation link -- 10^45 possibilities
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

// -- ERROR HANDLING for SUBSCRIBER TOKEN -- //
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // compiler casts `&sqlx::Error` into `&dyn Error`
        Some(&self.0)
    }
}

// HELPER for ERRORS: clearer error message logging / tracing -> used with `Debug` impl
pub fn error_chain_fmt(
    err: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", err)?;
    let mut current = err.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}
