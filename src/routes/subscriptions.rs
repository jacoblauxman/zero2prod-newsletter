use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
// temp fix re: warnings of unused vars
#[allow(dead_code)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
