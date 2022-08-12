use super::init::{Embedding, COLLECTION_NAME};
use crate::{embedder_host, prelude::*, qdrant_host};
use qdrant_client::{
  prelude::*,
  qdrant::{value::Kind, with_payload_selector::SelectorOptions, WithPayloadSelector},
};

#[tokio::main]
pub async fn search_blocking(query: impl ToString, limit: usize) -> Result<Vec<Verse>> {
  search(query, limit).await
}

pub async fn search(query: impl ToString, limit: usize) -> Result<Vec<Verse>> {
  let host = qdrant_host();
  let config = QdrantClientConfig {
    uri: format!("http://{}:6334", host),
    ..Default::default()
  };

  let client = QdrantClient::new(Some(config)).await?;
  let conn = Connection::open(SQLITE_DB)?;
  rusqlite::vtab::array::load_module(&conn)?;
  let query = query.to_string();

  let host = embedder_host();
  let response = reqwest::get(format!("http://{}:8000/embed?q={}", host, &query)).await?;
  let embedding: Embedding = serde_json::from_str(&response.text().await?)?;

  let search = SearchPoints {
    collection_name: COLLECTION_NAME.into(),
    vector: embedding.embedding,
    limit: limit as u64,
    with_payload: Some(WithPayloadSelector {
      selector_options: Some(SelectorOptions::Enable(true)),
    }),
    ..Default::default()
  };
  let result = client.search_points(search).await?;

  let cte: Vec<String> = result
    .result
    .iter()
    .enumerate()
    .map(|(i, r)| match r.payload.get("id") {
      Some(Value {
        kind: Some(Kind::IntegerValue(id)),
      }) => format!("({}, {})", *id, i + 1),
      _ => unreachable!(),
    })
    .collect();

  let query = format!("WITH cte(id, ord) AS (VALUES {}) SELECT DISTINCT verses.* FROM verses INNER JOIN cte ON cte.id = verses.id ORDER BY cte.ord", cte.join(","));
  let mut stmt = conn.prepare(&query)?;
  let verses = stmt
    .query_map([], Verse::parse_row)?
    .flat_map(|v| v)
    .collect();

  Ok(verses)
}
