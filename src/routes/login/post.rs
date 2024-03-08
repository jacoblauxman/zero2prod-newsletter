use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;
use crate::session_state::TypedSession;
use actix_web::http::header::LOCATION;
use actix_web::{web, HttpResponse};
use secrecy::Secret;
use sqlx::PgPool;
// can be built from `HttpResponse` and an err
// - can return as err from request handler (impls ResponseError) and reutrns to caller `HttpResponse` passed in to constructor
use actix_web::error::InternalError;
use actix_web_flash_messages::FlashMessage;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(skip(form, db_pool, session), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            // to avoid session fixation attacks  (seed users browser with 'known' session token BFORE log in - wait for auth and then IN)
            // rotates session token whenever user logs in:
            session.renew();
            session
                // .insert("user_id", user_id)
                .insert_user_id(user_id)
                .map_err(|err| login_redirect(LoginError::UnexpectedError(err.into())))?;
            // if something happens in serialization of `user_id` -> redirect to login w/ err message
            Ok(HttpResponse::SeeOther()
                // for setting redirect logic
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish())
        }
        Err(err) => {
            let err = match err {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
            };
            FlashMessage::error(err.to_string()).send();
            let res = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                // NOTE: we now use flash msgs from actix_web_flash_messages as middleware for creating / signing and setting cookie properties
                .finish();
            Err(InternalError::from_response(err, res))
        }
    }
}

// -- HELPERS for LOGIN -- //

// redirect to the login page with err msg
fn login_redirect(err: LoginError) -> InternalError<LoginError> {
    FlashMessage::error(err.to_string()).send();
    let res = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();
    InternalError::from_response(err, res)
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
