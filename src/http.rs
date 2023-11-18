use crate::candle::search;
use crate::model::Book;
#[cfg(feature = "rust_bert")]
use crate::search::search;
use anyhow::Result;
use axum::{extract::Path, routing::get, Form, Router};
use serde::Deserialize;

async fn query(form: Form<Query>) -> String {
  let verses = search(&form.q, 50, false).await.expect("failure to search");

  serde_json::to_string(&verses).expect("unable to serialize verses")
}

async fn chapter(Path((book_slug, chapter)): Path<(String, i32)>) -> String {
  let verses = Book::chapter(&book_slug, chapter).await.unwrap();
  serde_json::to_string(&verses).unwrap_or(format!(r#"{{error: "Could not deserialize verses."}}"#))
}

#[derive(Deserialize)]
struct Query {
  q: String,
}

pub async fn serve() -> Result<()> {
  let app = Router::new()
    .route("/q", get(query))
    .route("/chapter/:slug/:chapter", get(chapter));

  axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
    .serve(app.into_make_service())
    .await
    .unwrap();

  Ok(())
}
