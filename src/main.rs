use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // -- logging:
    // global logger setup - redirect all `log`'s events to our Subscriber (via tracing)
    LogTracer::init().expect("failed to set logger");
    // falling back to printing all spans at info-level or above if RUST_LOG env has not been set
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout); // output formatted spans to stdout

    // `with` provided via layer::SubscriberExt - extends trait for `Subscriber`, via sharing from tracing_subscriber crate
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    set_global_default(subscriber).expect("Failed to set subscriber"); // used by app to specify subscriber to be used to process spans

    let configuration = get_configuration().expect("Failed to read configuration file");
    let conn_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres Database");
    // config's values remove 'hardcoded' port value, now dynamic
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).expect("Failed to bind TcpListener to port");
    run(listener, conn_pool)?.await
}
