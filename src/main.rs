use sqlx::postgres::PgPool;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

use secrecy::ExposeSecret;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration file");
    let conn_pool =
        PgPool::connect_lazy(&configuration.database.connection_string().expose_secret())
            .expect("Failed to connect to Postgres Database");

    // let address = format!("127.0.0.1:{}", configuration.application_port);
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind TcpListener to port");
    run(listener, conn_pool)?.await
}
