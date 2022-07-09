use crate::search::search;
use actix_web::{get, App, HttpRequest, HttpResponse, HttpServer, Responder};
use qstring::QString;

#[get("/")]
async fn root(req: HttpRequest) -> impl Responder {
  let query = QString::from(req.query_string());

  let mut results = vec![];
  let mut body = String::from(HTML);

  if let Some(query) = query.get("q") {
    let verses = search(query, 10).await.expect("handle later");

    results = verses
      .into_iter()
      .map(|v| format!("{} {}:{} - {}", v.book, v.chapter, v.verse, v.content))
      .collect::<Vec<String>>();
  }

  body.push_str(&results.join("<br />"));
  body.push_str("</body></html>");
  HttpResponse::Ok()
    .content_type("text/html; charset=utf-8")
    .body(body)
}

#[actix_web::main]
pub async fn boot_server() -> std::io::Result<()> {
  println!("Running server.");
  HttpServer::new(|| App::new().service(root))
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

const HTML: &'static str = r#"
  <html>
    <body>
      <form action="/" method="get">
        <input type="text" name="q" />
      </form>
"#;
