use actix_web::{http::header::ContentType, web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    // we've encoded any potential err msg from LoginError to pass via query params in response redirect url!
    error: Option<String>,
}

pub async fn login_form(query: web::Query<QueryParams>) -> HttpResponse {
    let err_html = match query.0.error {
        None => "".into(),
        Some(err_msg) => format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&err_msg)),
        // `htmlescape` escapes html chars w/ entity-encoding (for XSS prevention)
    };

    HttpResponse::Ok()
        .content_type(ContentType::html())
        // .body(include_str!("login.html"))
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
                />
            </label>
            <label>Password
                <input
                type="password"
                placeholder="Enter Password"
                name="password"
                />
            </label>
            <button type="submit">Login</button>
        </form>
    </body>
    </html>"#,
        ))
}
