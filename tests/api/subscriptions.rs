use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails/transactional"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    let res = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(200, res.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_new_subscriber() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    // Mock::given(method("POST"))
    //     .and(path("/emails/transactional"))
    //     .respond_with(ResponseTemplate::new(200))
    //     .mount(&app.email_server)
    //     .await;

    // Act
    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription during testing");

    // Assert
    assert_eq!(saved.email, "mj_hohams@gmail.com");
    assert_eq!(saved.name, "mj hohams");
    assert_eq!(saved.status, "pending_confirmation");
}

#[tokio::test]
async fn subscribe_returns_400_when_form_data_missing() {
    // Arrange
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=mj%20hohams", "missing the email"),
        ("email=mj%5Fhohams%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (inv_body, err_msg) in test_cases {
        // Act
        let res = app.post_subscriptions(inv_body.into()).await;

        // Assert
        assert_eq!(
            400,
            res.status().as_u16(),
            // allows additional info re: err_msg on test failure
            "The API did not fail with 400 Bad Request when payload should have been {}",
            err_msg
        );
    }
}

#[tokio::test]
async fn subscribe_returns_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=mj%5Fhohams@gmail.com", "empty name"),
        ("name=hohams&email=", "empty email"),
        ("name=hohams&email=notavalidemailaddress", "invalid email"),
    ];

    for (body, descr) in test_cases {
        // Act
        let res = app.post_subscriptions(body.into()).await;

        // Assert
        assert_eq!(
            400,
            res.status().as_u16(),
            "The API did not return a 400 Bad Request when payload given was: {}",
            descr
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails/transactional"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    // on `drop` expect asserted
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_with_link() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails/transactional"))
        .respond_with(ResponseTemplate::new(200))
        // no expectation, test focuses on behavior elsewhere
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assert
    let email_req = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_req);
    // links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_fails_if_fatal_db_error() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj_hohams%40gmail.com";

    // sabotage db
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email;",)
        .execute(&app.db_pool)
        .await
        .unwrap();
    // Act
    let res = app.post_subscriptions(body.into()).await;
    // Assert
    assert_eq!(res.status().as_u16(), 500);
}
