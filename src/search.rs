use std::ops::Range;

use super::init::{Embedding, Verse, COLLECTION_NAME};
use crate::{embedder_host, init::SQLITE_DB, qdrant_host};
use anyhow::Result;
use qdrant_client::prelude::*;
use rusqlite::{params, Connection, Row};

#[tokio::main]
pub async fn search_blocking(query: impl ToString, limit: usize) -> Result<Vec<Verse>> {
  search(query, limit).await
}

pub async fn search(query: impl ToString, limit: usize) -> Result<Vec<Verse>> {
  let host = qdrant_host();
  println!("Qdrant host: {}", host);
  let config = QdrantClientConfig {
    uri: format!("http://{}:6334", host),
    ..Default::default()
  };

  let mut client = QdrantClient::new(Some(config)).await?;
  let conn = Connection::open(SQLITE_DB)?;
  let query = query.to_string();

  let host = embedder_host();
  println!("Embedder host: {}", host);
  let response = reqwest::get(format!("http://{}:8000/embed?q={}", host, &query)).await?;
  let embedding: Embedding = serde_json::from_str(&response.text().await?)?;

  let search = SearchPoints {
    collection_name: COLLECTION_NAME.into(),
    vector: embedding.embedding,
    limit: limit as u64,
    ..Default::default()
  };
  let result = client.search(search).await?;

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

      println!(
        "{} chapter {}, verse {}: {}",
        &verse.book, &verse.chapter, &verse.verse, &verse.content
      );

      verses.push(verse);
    }
  }

  Ok(verses)
}

impl Verse {
  pub fn query(slug: &str, chapter: u64, verses: Option<Range<u64>>) -> Result<Vec<Self>> {
    let conn = Connection::open(SQLITE_DB)?;
    let mut query = String::from(
      "SELECT id, content, book, verse FROM verses WHERE slug = (?1) AND chapter = (?2) ",
    );

    let parser = |row: &Row| {
      Ok(Verse {
        id: row.get(0)?,
        content: row.get(1)?,
        book: row.get(2)?,
        verse: row.get(3)?,
        slug: slug.to_string(),
        chapter: chapter,
      })
    };

    let mut stmt;
    let verses_iter = if let Some(verses) = verses {
      query.push_str("AND verse >= (?3) AND verse <= (?4) ORDER BY verse ASC");
      stmt = conn.prepare(&query)?;
      stmt.query_map(params![slug, chapter, verses.start, verses.end], parser)
    } else {
      query.push_str("ORDER BY verse ASC");
      stmt = conn.prepare(&query)?;
      stmt.query_map(params![slug, chapter], parser)
    }?;

    Ok(verses_iter.flat_map(|v| v).collect())
  }
}
