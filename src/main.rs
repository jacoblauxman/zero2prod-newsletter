use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
// async fn main() -> Result<(), std::io::Error> {
async fn main() -> anyhow::Result<()> {
    // initialize telemetry
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // build configuration and spin up app
    let configuration = get_configuration().expect("Failed to read configuration file");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}
