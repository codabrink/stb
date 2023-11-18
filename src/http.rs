use crate::candle::search;
use crate::model::Book;
#[cfg(feature = "rust_bert")]
use crate::search::search;
use anyhow::Result;
use axum::{
  extract::Path,
  http::Method,
  response::{IntoResponse, Json, Response},
  routing::{get, post},
  Router,
};
use axum_extra::extract::Multipart;
use tower_http::cors::{Any, CorsLayer};

async fn query(mut multipart: Multipart) -> Response {
  let mut q = String::new();
  while let Some(field) = multipart.next_field().await.unwrap() {
    let name = field.name().unwrap().to_string();
    match name.as_str() {
      "q" => q = field.text().await.expect("invalid field"),
      _ => unreachable!(),
    }
  }
  let verses = search(&q, 50, false).await.expect("failure to search");
  Json(verses).into_response()
}

async fn chapter(Path((book_slug, chapter)): Path<(String, i32)>) -> String {
  let verses = Book::chapter(&book_slug, chapter).await.unwrap();
  serde_json::to_string(&verses).unwrap_or(format!(r#"{{error: "Could not deserialize verses."}}"#))
}

pub async fn serve() -> Result<()> {
  let cors = CorsLayer::new()
    .allow_methods([Method::GET, Method::POST])
    .allow_origin(Any);

  let app = Router::new()
    .route("/q", post(query))
    .route("/chapter/:slug/:chapter", get(chapter))
    .layer(cors);

  axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
    .serve(app.into_make_service())
    .await?;

  Ok(())
}
