use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};
use wiremock::matchers::{any, method, path};
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

    let res = app.post_newsletters(newsletter_req_body).await;

    // Assert
    assert_eq!(res.status().as_u16(), 200);
    // Mock verifies on `Drop` the newsletter email hasn't been sent
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(method("POST"))
        .and(path("/emails/transactional"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_req_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    let res = app.post_newsletters(newsletter_req_body).await;

    // Assert
    assert_eq!(res.status().as_u16(), 200);
    // Mock verifies on `Drop` newsletter has been sent
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                        "content": {
                            "text": "Newsletter body as plain text",
                            "html": "<p>Newsletter body as HTML</p>",
                        } }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (invalid_body, err_msg) in test_cases {
        // Act
        let res = app.post_newsletters(invalid_body).await;

        // Assert
        assert_eq!(
            400,
            res.status().as_u16(),
            "The API did not fail with 400 Bad Request when payload was: {err_msg}"
        );
    }
}

// -- HELPERS for TESTS -- //

// uses public API of app (under test) to create unconfirmed sub
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=mj%20hohams&email=mj_hohams%40gmail.com";

    let _mock_guard = Mock::given(method("POST"))
        .and(path("/emails/transactional"))
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

    // inspect req's received by Mock Elastic Email server - retrieve confirmation link
    let email_req = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(&email_req)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    // re use of above helper with extra step to call confirmation link
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
