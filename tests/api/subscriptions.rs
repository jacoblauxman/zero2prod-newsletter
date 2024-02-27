use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";

    // Act
    let res = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(200, res.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription during testing");

    assert_eq!(saved.email, "mj_hohams@gmail.com");
    assert_eq!(saved.name, "mj hohams")
}

#[tokio::test]
async fn subscribe_returns_400_when_form_data_missing() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

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
    let client = reqwest::Client::new();
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
