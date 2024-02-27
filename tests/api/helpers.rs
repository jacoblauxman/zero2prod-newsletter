use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

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

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

// this helper creates an app process and additionally returns our needed port-bound app address and db pool's connection
pub async fn spawn_app() -> TestApp {
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

    // build new email client
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address");

    let timeout = configuration.email_client.timeout();

    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    let server = run(listener, db_pool.clone(), email_client).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp { address, db_pool }
}

// this helper creates a `for testing` database to use with our PgPool connection
async fn configure_database(configuration: &DatabaseSettings) -> PgPool {
    // create db instance
    let mut conn = PgConnection::connect_with(&configuration.without_db())
        .await
        .expect("Failed to connect to Postgres during db creation");
    conn.execute(format!(r#"CREATE DATABASE "{}""#, configuration.database_name).as_str())
        .await
        .expect("Failed to create test database");

    // migrate db using migrations dir
    let conn_pool = PgPool::connect_with(configuration.with_db())
        .await
        .expect("Failed to connect to Postgres during db migration");
    sqlx::migrate!("./migrations")
        .run(&conn_pool)
        .await
        .expect("Failed to migrate the db");

    conn_pool
}
