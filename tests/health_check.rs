use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

// logging initialization - once_cell ensures this static value init's only once in testing, but can still have access to TRACING post-init
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_level = "info".into();
    let subscriber_name = "test".into();

    // TEST_LOG flag check
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

// this helper creates an app process and additionally returns our needed port-bound app address and db pool's connection
async fn spawn_app() -> TestApp {
    // setup tracing: first time `init` invoked `TRACING` is executed - all others will skip
    Lazy::force(&TRACING);

    // at OS level - trying to bind port 0 will have OS scan for available port to then bind our app instance!
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration file");
    // - create a new logical db with unique name - run db migrations on it (randomized name via uuid)
    configuration.database.database_name = Uuid::new_v4().to_string();
    let db_pool = configure_database(&configuration.database).await;
    // utilize helper logic to return our created `for testing` PgPool

    let server = run(listener, db_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
}

// this helper creates a `for testing` database to use with our PgPool connection
pub async fn configure_database(configuration: &DatabaseSettings) -> PgPool {
    // create db instance
    let mut conn = PgConnection::connect(&configuration.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres during db creation");
    conn.execute(format!(r#"CREATE DATABASE "{}""#, configuration.database_name).as_str())
        .await
        .expect("Failed to create test database");

    // migrate db using migrations dir
    let conn_pool = PgPool::connect(&configuration.connection_string())
        .await
        .expect("Failed to connect to Postgres during db migration");
    sqlx::migrate!("./migrations")
        .run(&conn_pool)
        .await
        .expect("Failed to migrate the db");

    conn_pool
}

// SUBSCRIBE testing

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body = "name=mj%20hohams&email=mj%5Fhohams%40gmail.com";
    let res = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute POST request");

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
        let res = client
            .post(&format!("{}/subscriptions", &app.address))
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
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let res = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(res.status().is_success());
    assert_eq!(Some(0), res.content_length());
}
