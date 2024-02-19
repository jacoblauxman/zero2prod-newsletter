use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};

// async fn greet(req: HttpRequest) -> impl Responder {
//     let name = req.match_info().get("name").unwrap_or("werld");
//     format!("Hello {} !", &name)
// }

async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    HttpServer::new(|| {
        App::new()
        // .route("/", web::get().to(greet))
        // .route("/{name}", web::get().to(greet))
        .route("/health_check", web::get().to(health_check))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
