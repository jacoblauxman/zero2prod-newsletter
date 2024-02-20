use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // `init` calls `set_logger` -- falling back to print all logs info-level + above if RUST_LOG env has not been set
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = get_configuration().expect("Failed to read configuration file");
    let conn_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres Database");
    // config's values remove 'hardcoded' port value, now dynamic
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).expect("Failed to bind TcpListener to port");
    run(listener, conn_pool)?.await
}
