use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::{get_connection_pool, Application};
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
    pub email_server: MockServer,
    // for handling `/subscriptions/confirm` testing
    pub port: u16,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute POST request")
    }

    // method in order to get access to app port - needed to inject into links
    pub fn get_confirmation_links(&self, email_req: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_req.body).unwrap();

        // extract link from json
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap(); // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
}

// this helper creates an app process and additionally returns our needed port-bound app address and db pool's connection
pub async fn spawn_app() -> TestApp {
    // setup tracing: first time `init` invoked `TRACING` is executed - all others will skip
    Lazy::force(&TRACING);

    // launch mock server to stand in for Elastic Email API
    let email_server = MockServer::start().await;

    // randomize config for test isolation
    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration file");
        // randomize db name
        c.database.database_name = Uuid::new_v4().to_string();
        // randomize OS port
        c.application.port = 0;
        // use mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    // create + migrate db
    configure_database(&configuration.database).await;

    // launch application as background task
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application for testing");
    // need to extract port value after building test app
    let app_port = application.port();
    let address = format!("http://127.0.0.1:{}", app_port);

    let _ = tokio::spawn(application.run_until_stopped());
    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        port: app_port,
    }
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

// Confirmation links embedded in req to email API
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}
