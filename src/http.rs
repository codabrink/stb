use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn root() -> impl Responder {
  HttpResponse::Ok().body("Hello there")
}

#[post("/query")]
async fn query() -> impl Responder {
  HttpResponse::Ok().body("nothing yet")
}

#[actix_web::main]
pub async fn boot_server() -> std::io::Result<()> {
  HttpServer::new(|| {
    App::new().service(root).service(query)
    // .route("/hey", web::get().to(manual_hello))
  })
  .bind(("127.0.0.1", 8080))?
  .run()
  .await
}
