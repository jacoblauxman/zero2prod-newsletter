use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use actix_web::{
    http::header::{HeaderMap, HeaderValue},
    http::{header, StatusCode},
    web, HttpRequest, HttpResponse, ResponseError,
};
use anyhow::Context;
use base64::Engine;
use secrecy::Secret;
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

#[tracing::instrument(name = "Publish a newsletter",
    skip(body, db_pool, email_client, req),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    req: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    // we ensure we bubble up error when extracting headers + credentials from request
    let credentials = basic_authentication(req.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    // validate via db credential info
    // let user_id = validate_credentials(credentials, &db_pool).await?;
    let user_id = validate_credentials(credentials, &db_pool)
        .await
        // match on `AuthError` variants but pass ENTIRE err into constructors for `PublishError` variants ->
        // keeps top-level wrapper context preserved when logging via middleware
        .map_err(|err| match err {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(err.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(err.into()),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

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

// -- -- HELPERS for PUBLISH -- -- //

// set request's headers check and credential confirmation
fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("`Authorization` Header was missing")?
        .to_str()
        .context("`Authorization` Header was not a valid UTF8 string")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The auth scheme was not set to `Basic`")?;

    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode `Basic` credentials")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not a valid UTF8")?;

    // Split creds into two segments on ":" delimiter
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in `Basic` authorizaiton"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in `Basic` authorization"))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

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

    Ok(confirmed_subscribers)
}

// -- ERRORS for PUBLISH -- //

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication Failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    // `status_code` is invoked by default `error_response` implementation ->
    // providing `error_response` method impl,  swap from `status_code` (prior method / method name)
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut res = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_val = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                res.headers_mut()
                    // actix_web::http::header - provides collection of constants for names of standard HTTP headers (ie. WWW_AUTHENTICATE)
                    .insert(header::WWW_AUTHENTICATE, header_val);
                res
            }
        }
    }
}
