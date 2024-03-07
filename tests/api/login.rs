use crate::helpers::{assert_is_redirect_to, spawn_app};
use reqwest::header::HeaderValue;
use std::collections::HashSet;

#[tokio::test]
// `flash messages` - one time notifications (re: error msgs)
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let res = app.post_login(&login_body).await;

    // Assert
    // assert_eq!(res.status().as_u16(), 303);
    assert_is_redirect_to(&res, "/login");
    // let cookies: HashSet<_> = res.headers().get_all("Set-Cookie").into_iter().collect();
    // assert!(cookies.contains(&HeaderValue::from_str("_flash=Authentication failed").unwrap()));

    // we can utilize `reqwest` feature flag "cookies" to simplify our extraction of the cookies header:
    let flash_cookie = res
        .cookies()
        .find(|cookie| cookie.name() == "_flash")
        .unwrap();
    assert_eq!(flash_cookie.value(), "Authentication Failed");

    // Act #2
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication Failed</i></p>"#));
}
