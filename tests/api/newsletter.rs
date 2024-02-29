use crate::helpers::{spawn_app, TestApp};
use wiremock::matchers::{any, method};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_req_body = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    let res = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&newsletter_req_body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(res.status().as_u16(), 200);
    // Mock verifies on `Drop` we haven't sent the newsletter email
}

// uses public API of app (under test) to create unconfirmed sub
async fn create_unconfirmed_subscriber(app: &TestApp) {
    let body = "name=mj%20hohams&email=mj_hohams%40gmail.com";

    let _mock_guard = Mock::given(method("POST"))
        // .and(path("/transactional"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        // returns `MockGuard` guard obj
        // when `Drop`'d, `wiremock` tells `MockServer` to stop honoring specific mock behavior -> keeps mock behavior needed for test helper to `stay local`
        .mount_as_scoped(&app.email_server)
        // note: when `MockGuard` dropped, EAGERLY check expectations on the scope
        .await;

    // create unconfirmed subscriber in database
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();
}
