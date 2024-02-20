use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

pub fn run(listener: TcpListener, conn: PgPool) -> Result<Server, std::io::Error> {
    // wrap db connection (non-cloneable TCP connection with Postgres) in smart pointer (ARC) -- pointer to PgConnection
    let db_pool = web::Data::new(conn);
    // we have to capture `conn` from outer scope to use in innner scope
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            // register db conn as part of app state
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
