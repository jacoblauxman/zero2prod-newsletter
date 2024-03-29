use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    admin_dashboard, confirm, health_check, home, login, login_form, publish_newsletter, subscribe,
};

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, Secret};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

// holds server as well as app port
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        // let conn_pool = PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());
        let conn_pool = get_connection_pool(&configuration.database)
            .await
            .expect("Failed to connect to Postgres db");

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

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            conn_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;
        // allows saving of bound port to Application
        Ok(Self { port, server })
    }

    // reveals port for application initialization
    pub fn port(&self) -> u16 {
        self.port
    }

    // explicit fn that only returns when app is stopped
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

// wrapper type defined to retrieve URL in `subscribe` handler
// - note: retrieval from context in actix-web is type-based: using raw `String` would expose "conflicts"
pub struct ApplicationBaseUrl(pub String);

// -- -- RUN APP -- -- //

async fn run(
    listener: TcpListener,
    conn: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    // note: new error response (from std::io::Error)
    // context for base url - dependent on env
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    // context for email client's API
    let email_client = web::Data::new(email_client);
    // wrap db connection (non-cloneable TCP connection with Postgres) in smart pointer (ARC) -- pointer to PgConnection
    let db_pool = web::Data::new(conn);
    // for session token and setup of session storage
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    // cookie storage + flash msg handling
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    // instantiate our Redis session store from uri conn string
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    // we have to capture `conn` from outer scope to use in innner scope
    let server = HttpServer::new(move || {
        App::new()
            // middleware is added using `wrap` on `App`
            // allows for flash msgs re: signed cookies
            .wrap(message_framework.clone())
            // allows access to session via Redis
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            )) // session wrapper for entire app
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/admin/dashboard", web::get().to(admin_dashboard))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/health_check", web::get().to(health_check))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            // register db conn as part of app state
            .app_data(db_pool.clone())
            // since EC has two data fields (base_url and sender) along with Client, share (wrapped via Ac) amongst all App instances (one per thread)
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            // injecting secret used by HMAC's to app state
            .app_data(web::Data::new(HmacSecret(hmac_secret.clone())))
    })
    .listen(listener)?
    .run();

    Ok(server)
}

// helper - builds connection to pg pool
pub async fn get_connection_pool(configuration: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .connect_with(configuration.with_db())
        .await
}

// wrapper type to avoid conflicts re: use of `Secret<String>` as type injected for HMAC value (registering another `Secret<String>` could override)
#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);
