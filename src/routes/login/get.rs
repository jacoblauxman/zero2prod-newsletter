use actix_web::{cookie::Cookie, http::header::ContentType, HttpRequest, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

// no longer need access to raw req
pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    // UPDATE: using cookies now as opposed to HMAC + query params
    // let err_html = match req.cookie("_flash") {
    //     None => "".into(),
    //     Some(cookie) => {
    //         format!("<p><i>{}</i></p>", cookie.value())
    //     }
    // };

    let mut err_html = String::new();
    for msg in flash_messages
        .iter()
        .filter(|msg| msg.level() == Level::Error)
    {
        writeln!(err_html, "<p><i>{}</i></p>", msg.content()).unwrap();
    }

    HttpResponse::Ok()
        .content_type(ContentType::html())
        // adjust req handler to set "Max-Age" property of login cookie to 0 (expires immediately / clears)
        // .cookie(Cookie::build("_flash", "").max_age(Duration::ZERO).finish())
        // clarified by helper method `add_removal_cookie` used below from storing `res`
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

    // NOTE: actix_web_flash_messages now handles cookie setting and properties as well as removal!
    // res.add_removal_cookie(&Cookie::new("_flash", "")).unwrap(); // does the same as setting "Max-Age" to 0
    // res
}
