use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
// `flash messages` - one time notifications (re: error msgs)
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act - try to login
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let res = app.post_login(&login_body).await;

    // Assert
    assert_is_redirect_to(&res, "/login"); // this checks both status code and "path" of re-direct

    // we can utilize `reqwest` feature flag "cookies" to simplify our extraction of the cookies header:
    // let flash_cookie = res
    //     .cookies()
    //     .find(|cookie| cookie.name() == "_flash")
    //     .unwrap();
    // assert_eq!(flash_cookie.value(), "Authentication Failed");

    // Act 2 - follow redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>Authentication Failed</i></p>"));

    // Act 3 - reload login page
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains("Authentication Failed"));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    // Arrange
    let app = spawn_app().await;

    // Act 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    // Assert 1
    let res = app.post_login(&login_body).await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    // Act 2 - Follow redirect
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));
}
