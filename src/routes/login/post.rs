use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;
use crate::startup::HmacSecret;
use actix_web::cookie::Cookie;
use actix_web::http::header::LOCATION;
use actix_web::{web, HttpResponse};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
// can be built from `HttpResponse` and an err
// - can return as err from request handler (impls ResponseError) and reutrns to caller `HttpResponse` passed in to constructor
use actix_web::error::InternalError;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(skip(form, db_pool), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
pub async fn login(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    // injecting HMAC value (wrapper around Secret<String>)
    // secret: web::Data<HmacSecret>,
    // UPDATE: query params are not as secure as `flash messages` via cookies - HMAC no longer needed
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                // for setting redirect logic
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(err) => {
            let err = match err {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
            };

            // let query_str = format!("error={}", urlencoding::Encoded::new(err.to_string()));

            // let hmac_tag = {
            //     let mut mac =
            //         Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes())
            //             .unwrap();
            //     mac.update(query_str.as_bytes());
            //     mac.finalize().into_bytes()
            // };

            // let res = HttpResponse::SeeOther()
            //     .insert_header((LOCATION, format!("/login?{}&tag={:x}", query_str, hmac_tag)))
            //     .finish();
            // Err(InternalError::from_response(err, res))
            //
            let res = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                // COOKIES: example of setting cookies into header via actix-web
                // .insert_header(("Set-Cookie", format!("_flash={err}")))
                .cookie(Cookie::new("_flash", err.to_string()))
                .finish();
            Err(InternalError::from_response(err, res))
        }
    }

    // let user_id = validate_credentials(credentials, &db_pool)
    //     .await
    //     .map_err(|err| match err {
    //         AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
    //         AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
    //     })?;
    // tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    // Ok(HttpResponse::SeeOther()
    //     .insert_header((LOCATION, "/"))
    //     .finish())
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

// impl ResponseError for LoginError {
//     // `error_response` via `ResponseError` provided by actix_web populates body using `Display` representation of err from handler
//     fn error_response(&self) -> HttpResponse {
//         let encoded_err = urlencoding::Encoded::new(self.to_string());
//         let query_str = format!("error={}", encoded_err);

//         let secret: &[u8] = todo!();
//         let hmac_tag = {
//             let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
//             mac.update(query_str.as_bytes());
//             mac.finalize().into_bytes()
//         };

//         HttpResponse::build(self.status_code())
//             .insert_header((LOCATION, format!("/login?{}&tag={:x}", query_str, hmac_tag)))
//             .finish()
//     }
//     fn status_code(&self) -> StatusCode {
//         match self {
//             LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
//             LoginError::AuthError(_) => StatusCode::UNAUTHORIZED,
//         }
//     }
// }
