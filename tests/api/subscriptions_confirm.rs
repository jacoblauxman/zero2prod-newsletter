use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn link_returned_by_subscribe_returns_200_if_called() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails/transactional"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_req = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_req);

    // Act
    let res = reqwest::get(confirmation_links.html).await.unwrap();

    // Assert
    assert_eq!(res.status().as_u16(), 200);
}

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_400() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let res = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    // Assert
    assert_eq!(res.status().as_u16(), 400);
}

#[tokio::test]
async fn clicking_confirmation_link_confirms_subscriber() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    Mock::given(method("POST"))
        .and(path("/emails/transactional"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_req = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_req);

    // Act
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Assert
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription during testing");

    assert_eq!(saved.email, "mj_hohams@gmail.com");
    assert_eq!(saved.name, "mj hohams");
    assert_eq!(saved.status, "confirmed");
}
