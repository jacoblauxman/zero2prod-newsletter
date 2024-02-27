use actix_web::{web, HttpResponse};

// for ensuring `subscription_token` query param via `Query`
#[derive(serde::Deserialize)]
pub struct Paramaters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(_parameters))]
pub async fn confirm(_parameters: web::Query<Paramaters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
