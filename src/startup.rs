use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::email_client::EmailClient;

pub fn run(
    listener: TcpListener,
    conn: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let email_client = web::Data::new(email_client);
    // wrap db connection (non-cloneable TCP connection with Postgres) in smart pointer (ARC) -- pointer to PgConnection
    let db_pool = web::Data::new(conn);
    // we have to capture `conn` from outer scope to use in innner scope
    let server = HttpServer::new(move || {
        App::new()
            // middleware is added using `wrap` on `App`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // register db conn as part of app state
            .app_data(db_pool.clone())
            // since EC has two data fields (base_url and sender) along with Client, share (wrapped via Ac) amongst all App instances (one per thread)
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
