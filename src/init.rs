use anyhow::Result;
use glob::glob;
use qdrant_client::{prelude::*, qdrant::Distance};
use regex::Regex;
use rusqlite::{params, Connection};
use serde::Deserialize;

pub const SQLITE_DB: &'static str = "db.sqlite";
pub const COLLECTION_NAME: &'static str = "verses";

#[tokio::main]
pub async fn rebuild_sql() -> Result<()> {
  let _ = std::fs::remove_file(SQLITE_DB);
  let conn = Connection::open(SQLITE_DB)?;

  conn.execute(
    "
    CREATE TABLE verses (
      id INTEGER PRIMARY KEY,
      book       TEXT NOT NULL,
      chapter    INTEGER NOT NULL,
      verse      INTEGER NOT NULL,
      content    TEXT NOT NULL
    )",
    [],
  )?;

  let chapter_regex = Regex::new(r"Chapter\s(\d+)\.")?;

  for entry in glob("bible/eng-web_*.txt").expect("Failed to read Bible directory") {
    let entry = entry?;
    let text_file = std::fs::read_to_string(&entry)?;
    let lines: Vec<&str> = text_file.split('\n').collect();

    let book = &lines[0][..lines[0].len() - 1];
    let chapter = &chapter_regex.captures(&lines[1]).unwrap()[1];

    println!(
      "Book: {}, Chapter: {}, file name: {}",
      book,
      chapter,
      entry.file_name().unwrap().to_string_lossy()
    );

    let mut verse = 1;
    for line in &lines[2..] {
      if line.trim().is_empty() {
        continue;
      }

      conn.execute(
        "INSERT INTO verses (book, chapter, verse, content) VALUES (?1, ?2, ?3, ?4)",
        params![book, chapter, verse, line],
      )?;

      verse += 1;
    }
  }

  Ok(())
}

pub struct Verse {
  pub id: u64,
  pub verse: u64,
  pub chapter: u64,
  pub book: String,
  pub content: String,
}

#[derive(Deserialize)]
pub struct Embedding {
  pub embedding: Vec<f32>,
}

#[tokio::main]
pub async fn rebuild_vector() -> Result<()> {
  let mut client = QdrantClient::new(None).await?;

  if client.has_collection(COLLECTION_NAME).await? {
    client.delete_collection(COLLECTION_NAME).await?;
  }

  client
    .create_collection(CreateCollection {
      collection_name: COLLECTION_NAME.into(),
      distance: Distance::Dot.into(),
      vector_size: 768,
      ..Default::default()
    })
    .await?;

  let conn = Connection::open(SQLITE_DB)?;
  let mut stmt = conn.prepare("SELECT id, book, chapter, verse, content FROM verses")?;

  let verse_iter = stmt.query_map([], |row| {
    Ok(Verse {
      id: row.get(0)?,
      book: row.get(1)?,
      chapter: row.get(2)?,
      verse: row.get(3)?,
      content: row.get(4)?,
    })
  })?;

  let mut points: Vec<PointStruct> = Vec::new();

  for verse in verse_iter {
    let verse = verse?;
    let response = reqwest::get(format!("http://localhost:8000/embed?q={}", verse.content)).await?;

    let embedding: Embedding = serde_json::from_str(&response.text().await?)?;
    println!(
      "Gathered embedding for {} Chapter {} Verse {}",
      verse.book, verse.chapter, verse.verse
    );

    points.push(PointStruct::new(
      verse.id,
      embedding.embedding,
      Payload::new(),
    ));
  }

  println!("Upserting points...");
  client.upsert(COLLECTION_NAME, points).await?;
  println!("Upserted.");

  Ok(())
}
