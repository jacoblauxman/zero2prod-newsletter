use std::net::TcpListener;
use sqlx::{PgConnection, Connection};
use zero2prod::configuration::get_configuration;


// this helper creates app process and additionally returns our needed port-bound app address
fn spawn_app() -> String {
    // at OS level - trying to bind port 0 will have OS scan for available port to then bind our app instance!
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::startup::run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

// SUBSCRIBE testing

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let app_addr = spawn_app();

    let config = get_configuration().expect("Failed to read configuration file");
    let conn_string = config.database.connection_string();
    let mut conn = PgConnection::connect(&conn_string)
    .await
    .expect("Failed to connect to Postgres Database");

    let client = reqwest::Client::new();

    // Act
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";
    let res = client
        .post(&format!("{}/subscriptions", &app_addr))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute POST request");

    // Assert
    assert_eq!(200, res.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",).fetch_one(&mut conn).await.expect("Failed to fetch saved subscription during testing");

    assert_eq!(saved.email, "mj_hohams@gmail.com");
    assert_eq!(saved.name, "mj hohams")
}

#[tokio::test]
async fn subscribe_returns_400_when_form_data_missing() {
    // Arrange
    let app_addr = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=mj%20hohams", "missing the email"),
        ("email=mj_hohams%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (inv_body, err_msg) in test_cases {

        // Act
        let res = client
            .post(&format!("{}/subscriptions", &app_addr))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(inv_body)
            .send()
            .await
            .expect("Failed to execute POST request");

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

//

//

// HEALTH CHECK testing

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let addr = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let res = client
        .get(&format!("{}/health_check", &addr))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(res.status().is_success());
    assert_eq!(Some(0), res.content_length());
}
