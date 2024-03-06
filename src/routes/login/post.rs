use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;
use actix_web::http::{header::LOCATION, StatusCode};
use actix_web::{web, HttpResponse, ResponseError};
use hmac::{Hmac, Mac};
use secrecy::Secret;
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(skip(form, db_pool), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, LoginError> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    // match validate_credentials(credentials, &db_pool).await {
    //     Ok(user_id) => {
    //         tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    //         HttpResponse::SeeOther()
    //             .insert_header((LOCATION, "/"))
    //             .finish()
    //     }
    //     Err(_) => todo!(),
    // }
    let user_id = validate_credentials(credentials, &db_pool)
        .await
        .map_err(|err| match err {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}

// -- ERRORS for LOGIN -- //
#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication Failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    // `error_response` via `ResponseError` provided by actix_web populates body using `Display` representation of err from handler
    fn error_response(&self) -> HttpResponse {
        let encoded_err = urlencoding::Encoded::new(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((LOCATION, format!("/login?error={}", encoded_err)))
            .finish()
    }
    fn status_code(&self) -> StatusCode {
        match self {
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }
}
