use crate::search::search;
use actix_web::{get, App, HttpRequest, HttpResponse, HttpServer, Responder};
use qstring::QString;

#[get("/")]
async fn root() -> impl Responder {
  HttpResponse::Ok().body("Hello there")
}

#[get("/query")]
async fn query(req: HttpRequest) -> impl Responder {
  let query = QString::from(req.query_string());
  if let Some(query) = query.get("q") {
    let verses = search(query, 10).await.expect("handle later");

    let body = verses
      .into_iter()
      .map(|v| format!("{} {}:{} - {}", v.book, v.chapter, v.verse, v.content))
      .collect::<Vec<String>>()
      .join("\n");

    return HttpResponse::Ok().body(body);
  }
  HttpResponse::Ok().body("nothing yet")
}

#[actix_web::main]
pub async fn boot_server() -> std::io::Result<()> {
  println!("Running server.");
  HttpServer::new(|| {
    App::new().service(root).service(query)
    // .route("/hey", web::get().to(manual_hello))
  })
  .bind(("127.0.0.1", 8080))?
  .run()
  .await
}
