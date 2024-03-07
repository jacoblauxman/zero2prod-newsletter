// use crate::startup::HmacSecret;
use actix_web::{http::header::ContentType, web, HttpRequest, HttpResponse};
// use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

// #[derive(serde::Deserialize)]
// pub struct QueryParams {
//     // we've encoded any potential err msg from LoginError to pass via query params in response redirect url!
//     error: String,
//     tag: String, // for our HMAC tag
// }

// impl QueryParams {
//     // method for returning err string for query params if msg auth code matches expectations -> otherwise ERR (ie. )
//     fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
//         let tag = hex::decode(self.tag)?; // our hmac tag was encoded as hexidecimal, decode back to bytes!
//         let query_str = format!("error={}", urlencoding::Encoded::new(&self.error));

//         let mut mac =
//             Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
//         mac.update(query_str.as_bytes());
//         // confirms tag matches hmac secret as expected
//         mac.verify_slice(&tag)?;

//         Ok(self.error)
//     }
// }

pub async fn login_form(
    // query: Option<web::Query<QueryParams>>,
    // secret: web::Data<HmacSecret>,
    req: HttpRequest,
) -> HttpResponse {
    // UPDATE: using cookies now as opposed to HMAC + query params
    let err_html = match req.cookie("_flash") {
        None => "".into(),
        Some(cookie) => {
            format!("<p><i>{}</i></p>", cookie.value())
        }
    };

    // our `query` is now optional - however, if present needs ALL fields (error and hmac tag)
    // let err_html = todo!();
    //     match query {
    //     None => "".into(),
    //     Some(query) => match query.0.verify(&secret) {
    //         Ok(err) => {
    //             format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&err))
    //             // `htmlescape` escapes html chars w/ entity-encoding (for XSS prevention)
    //         }
    //         // this error is caused to mismatch in HMAC tag secret and request `query` (after verification)
    //         Err(err) => {
    //             tracing::warn!(error.message = %err,  error.cause_chain = ?err, "Failed to verify query params using the HMAC tag");
    //             "".into()
    //         }
    //     },
    // };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
    {err_html}
    <form action="/login" method="post">
        <label>Username
            <input
                type="text"
                placeholder="Enter Username"
                name="username"
            >
        </label>
        <label>Password
            <input
                type="password"
                placeholder="Enter Password"
                name="password"
            >
        </label>
        <button type="submit">Login</button>
    </form>
</body>
</html>"#,
        ))
}
