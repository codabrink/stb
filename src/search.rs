use super::init::{Embedding, COLLECTION_NAME};
use crate::{embedder_host, prelude::*, qdrant_host};
use qdrant_client::prelude::*;

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
  let query = query.to_string();

  let host = embedder_host();
  let response = reqwest::get(format!("http://{}:8000/embed?q={}", host, &query)).await?;
  let embedding: Embedding = serde_json::from_str(&response.text().await?)?;

  let search = SearchPoints {
    collection_name: COLLECTION_NAME.into(),
    vector: embedding.embedding,
    limit: limit as u64,
    ..Default::default()
  };
  let result = client.search_points(search).await?;

  let mut verses = Vec::with_capacity(limit);

  for result in result.result {
    if let Some(point_id::PointIdOptions::Num(v)) = result.id.unwrap().point_id_options {
      let verse: Verse = conn.query_row(
        "SELECT content, book, slug, chapter, verse FROM verses WHERE id = (?1)",
        params![v],
        |row| {
          Ok(Verse {
            id: v,
            content: row.get(0)?,
            book: row.get(1)?,
            slug: row.get(2)?,
            chapter: row.get(3)?,
            verse: row.get(4)?,
          })
        },
      )?;

      verses.push(verse);
    }
  }

  Ok(verses)
}
