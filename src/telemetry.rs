use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

// note: p.119 REVIEW
use tracing_subscriber::fmt::MakeWriter;

// using `impl..` return type to help generalize complicated specifics - mainly needs to be Subscriber with Sync + Send to pass along
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Sync + Send
where
    // See more: https://doc.rust-lang.org/nomicon/hrtb.html
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // RUST_LOG env - defaults to `info` level unless set
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

// Register subscriber as global default to process span data (only called once)
pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
    LogTracer::init().expect("Failed to set logger via LogTracer");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
