use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};
use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

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

// form submission handling / orchestration
#[tracing::instrument(
    name = "Adding a new subscriber",
    // tracing by default captures all args to fn, skip used to omit info in log
    skip(form, db_pool, email_client, base_url),
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
    // retrieve email client from app state
    email_client: web::Data<EmailClient>,
    // app env base'd
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    // get subscriber data from form input
    let new_subscriber = form.0.try_into()?;
    // `begin` acquires connection from the db's pool to kick off transaction -- provides way to convert multi-steps of db interaction into 'all-or-nothing'
    // let mut transaction = db_pool.begin().await?;
    let mut transaction = db_pool.begin().await.map_err(SubscribeError::PoolError)?;
    // get subscriber's id from inserting into db, return `500` if fails
    // let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber).await?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .map_err(SubscribeError::InsertSubscriberError)?;
    // rand gen'd confirmation token
    let subscription_token = generate_subscription_token();
    // store subscriber's token in db associated to sub's id, `500` if fails
    store_token(&mut transaction, subscriber_id, &subscription_token).await?;
    // finalize db transaction/commit and return conn to db pool, `500` if fails
    // transaction.commit().await?;
    transaction
        .commit()
        .await
        .map_err(SubscribeError::TransactionCommitError)?;
    // send email via external API service, `500` if fails
    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await?;

    Ok(HttpResponse::Ok().finish())
}

// #[derive(Debug)]
// struct SubscribeError {}
pub enum SubscribeError {
    ValidationError(String),
    // DatabaseError(sqlx::Error),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error),
    PoolError(sqlx::Error),
    InsertSubscriberError(sqlx::Error),
    TransactionCommitError(sqlx::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "Failed to create a new subscriber")
        match self {
            SubscribeError::ValidationError(err) => write!(f, "{err}"),
            // SubscribeError::DatabaseError(_) => write!(f, "???"),
            SubscribeError::PoolError(_) => {
                write!(
                    f,
                    "Failed to acquire a Postgres connection from the db pool"
                )
            }
            SubscribeError::InsertSubscriberError(_) => {
                write!(f, "Failed to insert a new subscriber in the database")
            }
            SubscribeError::TransactionCommitError(_) => {
                write!(
                    f,
                    "Failed to commit SQL transaction to store new subscriber"
                )
            }
            SubscribeError::StoreTokenError(_) => write!(
                f,
                "Failed to store the confirmation token for a new subscriber"
            ),
            SubscribeError::SendEmailError(_) => write!(f, "Failed to send confirmation email"),
        }
    }
}

impl std::error::Error for SubscribeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // &str / String does not implement `Error` - make 'root cause'
            SubscribeError::ValidationError(_) => None,
            // SubscribeError::DatabaseError(err) => Some(err),
            SubscribeError::StoreTokenError(err) => Some(err),
            SubscribeError::SendEmailError(err) => Some(err),
            SubscribeError::PoolError(err) => Some(err),
            SubscribeError::InsertSubscriberError(err) => Some(err),
            SubscribeError::TransactionCommitError(err) => Some(err),
        }
    }
}

// default implementation returns `500` - control flow provided for `SubscribeError` variations
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::PoolError(_)
            | SubscribeError::InsertSubscriberError(_)
            | SubscribeError::TransactionCommitError(_)
            | SubscribeError::SendEmailError(_)
            | SubscribeError::StoreTokenError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<reqwest::Error> for SubscribeError {
    fn from(err: reqwest::Error) -> Self {
        Self::SendEmailError(err)
    }
}

// impl From<sqlx::Error> for SubscribeError {
//     fn from(err: sqlx::Error) -> Self {
//         Self::DatabaseError(err)
//     }
// }

impl From<StoreTokenError> for SubscribeError {
    fn from(err: StoreTokenError) -> Self {
        Self::StoreTokenError(err)
    }
}

impl From<String> for SubscribeError {
    fn from(err: String) -> Self {
        Self::ValidationError(err)
    }
}

// send confirmation email to new subscriber
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
            new_subscriber.email,
            "Welcome!",
            &html_content,
            &text_content,
        )
        .await
}

// insert subscriber into database
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

    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

// insert token into db
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
        tracing::error!("Failed to execute query: {:?}", err);
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

// #[derive(Debug)] // we need to derive Debug and implement Display for our custom error type (no macro for Display)
pub struct StoreTokenError(sqlx::Error);

// remove derive - make message more explicit
impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "{}\nCaused by:\n\t{}", self, self.0)
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

fn error_chain_fmt(
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
