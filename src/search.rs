use super::init::{Embedding, Verse, COLLECTION_NAME};
use crate::init::SQLITE_DB;
use anyhow::Result;
use qdrant_client::prelude::*;
use rusqlite::{params, Connection};

#[tokio::main]
pub async fn search_blocking(query: impl ToString, limit: usize) -> Result<Vec<Verse>> {
  search(query, limit).await
}

pub async fn search(query: impl ToString, limit: usize) -> Result<Vec<Verse>> {
  let mut client = QdrantClient::new(None).await?;
  let conn = Connection::open(SQLITE_DB)?;
  let query = query.to_string();

  let response = reqwest::get(format!("http://localhost:8000/embed?q={}", &query)).await?;
  let embedding: Embedding = serde_json::from_str(&response.text().await?)?;

  let result = client
    .search(COLLECTION_NAME, embedding.embedding, limit as u64)
    .await?;

  let mut verses = Vec::with_capacity(limit);

  for result in result.result {
    if let Some(point_id::PointIdOptions::Num(v)) = result.id.unwrap().point_id_options {
      let verse: Verse = conn.query_row(
        "SELECT content, book, chapter, verse FROM verses WHERE id = (?1)",
        params![v],
        |row| {
          Ok(Verse {
            id: v,
            content: row.get(0)?,
            book: row.get(1)?,
            chapter: row.get(2)?,
            verse: row.get(3)?,
          })
        },
      )?;

      println!(
        "{} chapter {}, verse {}: {}",
        &verse.book, &verse.chapter, &verse.verse, &verse.content
      );

      verses.push(verse);
    }
  }

  Ok(verses)
}
