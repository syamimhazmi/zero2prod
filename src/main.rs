use actix_web::{web, App, HttpServer, Responder, HttpResponse};

async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    HttpServer::new( || {
        App::new()
            .route("/healt-check", web::get().to(health_check))
    })
        .bind("127.0.0.1:9001")?
        .run()
        .await
}