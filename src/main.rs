use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::{run, Application};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration file");
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await?;
    Ok(())
}

// #[tokio::main]
// async fn main() -> Result<(), std::io::Error> {
//     let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
//     init_subscriber(subscriber);

//     let configuration = get_configuration().expect("Failed to read configuration file");
//     let conn_pool = PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());

//     // build `EmailClient` from `configuration`
//     let timeout = configuration.email_client.timeout();
//     let sender_email = configuration
//         .email_client
//         .sender()
//         .expect("Invalid sender email address");
//     let email_client = EmailClient::new(
//         configuration.email_client.base_url,
//         sender_email,
//         configuration.email_client.authorization_token,
//         timeout,
//     );

//     // let address = format!("127.0.0.1:{}", configuration.application_port);
//     let address = format!(
//         "{}:{}",
//         configuration.application.host, configuration.application.port
//     );

//     let listener = TcpListener::bind(address).expect("Failed to bind TcpListener to port");
//     run(listener, conn_pool, email_client)?.await
// }
