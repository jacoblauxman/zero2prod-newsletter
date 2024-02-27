use crate::helpers::spawn_app;
use reqwest::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn link_returned_by_subscribe_returns_200_if_called() {
    // Arrange
    let app = spawn_app().await;
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    Mock::given(method("POST"))
        // .and(path("/transactional"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_req = &app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_req.body).unwrap();
    // extracting link from req fields
    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|link| *link.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };
    let raw_confirmation_link = &get_link(&body["HtmlBody"].as_str().unwrap());
    let mut confirmation_link = Url::parse(raw_confirmation_link).unwrap();
    // confirm host API call
    assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
    // then rewrite url to include port
    confirmation_link.set_port(Some(app.port)).unwrap();

    // Act
    let res = reqwest::get(confirmation_link).await.unwrap();

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
