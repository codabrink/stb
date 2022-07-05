use crate::init::SQLITE_DB;

use super::init::{Embedding, Verse, COLLECTION_NAME};
use anyhow::Result;
use qdrant_client::prelude::*;
use rusqlite::{params, Connection};

#[tokio::main]
pub async fn search(query: impl ToString) -> Result<()> {
  let mut client = QdrantClient::new(None).await?;
  let conn = Connection::open(SQLITE_DB)?;
  let query = query.to_string();

  let response = reqwest::get(format!("http://localhost:8000/embed?q={}", &query)).await?;
  let embedding: Embedding = serde_json::from_str(&response.text().await?)?;

  let result = client
    .search(COLLECTION_NAME, embedding.embedding, 10)
    .await?;

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
      // assert!(verse.content.len() == 0);
      println!(
        "{} chapter {}, verse {}: {}",
        verse.book, verse.chapter, verse.verse, verse.content
      );
    }
  }

  Ok(())
}
