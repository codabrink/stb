use crate::prelude::*;

use futures::{stream, StreamExt};
use glob::glob;
use lazy_static::lazy_static;
use qdrant_client::{prelude::*, qdrant::Distance};
use regex::Regex;
use reqwest::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;

pub const COLLECTION_NAME: &'static str = "verses";

lazy_static! {
  static ref SPLIT_REGEX: Regex = Regex::new(r",|\.").unwrap();
}

#[tokio::main]
pub async fn rebuild_sql() -> Result<()> {
  let _ = std::fs::remove_file(SQLITE_DB);
  let conn = Connection::open(SQLITE_DB)?;

  conn.execute(
    "
    CREATE TABLE verses (
      id         INTEGER PRIMARY KEY,
      slug       TEXT NOT NULL,
      book       TEXT NOT NULL,
      chapter    INTEGER NOT NULL,
      verse      INTEGER NOT NULL,
      content    TEXT NOT NULL
    );
    CREATE INDEX idx_verses ON verses (slug, chapter, verse);
    ",
    [],
  )?;

  conn.execute(
    "
    CREATE TABLE books (
      id       INTEGER PRIMARY KEY,
      slug     TEXT NOT NULL,
      name     TEXT NOT NULL,
      chapters INTEGER NOT NULL,
      ord      INTEGER NOT NULL
    );
    CREATE INDEX idx_books ON books (order, slug);
  ",
    [],
  )?;

  let chapter_regex = Regex::new(r"Chapter\s(\d+)\.")?;

  let mut order = 0;

  for entry in glob("bible/eng-web_*.txt").expect("Failed to read Bible directory") {
    let entry = entry?;
    let file_name = entry.file_name().unwrap().to_string_lossy().to_string();

    let slug = file_name.split('_').nth(2).unwrap();

    let text_file = std::fs::read_to_string(&entry)?;
    let lines: Vec<&str> = text_file.split('\n').collect();

    let book = &lines[0][..lines[0].len() - 1];
    let chapter = &chapter_regex.captures(&lines[1]).unwrap()[1];

    {
      let count: u64 = conn.query_row(
        "SELECT COUNT(*) FROM books WHERE slug = (?1)",
        params![slug],
        |r| r.get(0),
      )?;
      if count == 0 {
        conn.execute(
          "INSERT INTO books (slug, name, chapters, ord) VALUES (?1, ?2, ?3, ?4);",
          params![slug, book, 1, order],
        )?;
        order += 1;
      } else {
        conn.execute(
          "UPDATE books SET chapters = chapters + 1 WHERE slug = (?1)",
          params![slug],
        )?;
      }
    }

    println!(
      "Book: {} ({}), Chapter: {}, file name: {}",
      book,
      slug,
      chapter,
      entry.file_name().unwrap().to_string_lossy()
    );

    let mut verse = 1;
    for line in &lines[2..] {
      if line.trim().is_empty() {
        continue;
      }

      conn.execute(
        "INSERT INTO verses (book, slug, chapter, verse, content) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![book, slug, chapter, verse, line],
      )?;

      verse += 1;
    }
  }

  Ok(())
}

#[derive(Deserialize)]
pub struct Embedding {
  pub embedding: Vec<f32>,
}

#[tokio::main]
pub async fn rebuild_vector() -> Result<()> {
  let client = QdrantClient::new(None).await?;

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
  let mut stmt = conn.prepare("SELECT id, book, slug, chapter, verse, content FROM verses")?;

  let verse_iter = stmt.query_map([], |row| {
    Ok(Verse {
      id: row.get(0)?,
      book: row.get(1)?,
      slug: row.get(2)?,
      chapter: row.get(3)?,
      verse: row.get(4)?,
      content: row.get(5)?,
    })
  })?;

  let reqwest_client = reqwest::Client::new();
  let mut bodies = stream::iter(verse_iter)
    .map(|v| {
      let client = &reqwest_client;
      async move {
        let v = v?;
        let mut embeddings = Vec::new();

        embeddings.push(embed(&v.content, client).await?);
        let splits: Vec<String> = SPLIT_REGEX
          .split(&v.content)
          .map(|s| {
            s.chars()
              .filter(|c| *c == ' ' || *c == '\'' || c.is_ascii_alphanumeric())
              .collect()
          })
          .collect();

        if splits.len() > 1 {
          for split in &splits {
            if split.len() < 8 {
              continue;
            }
            println!("SPLIT: {}", &split.trim());
            embeddings.push(embed(split.trim(), client).await?);
          }
        }

        for i in 1..(splits.len() - 1) {
          let subverse = splits[i..]
            .iter()
            .map(|s| s.trim())
            .collect::<Vec<&str>>()
            .join(" ");

          println!("SUBVERSE: {}", &subverse);
          embeddings.push(embed(&subverse, client).await?);
        }

        println!(
          "Gathered embedding for {} Chapter {} Verse {} ({} splits)",
          v.book,
          v.chapter,
          v.verse,
          &splits.len()
        );

        anyhow::Ok((v, embeddings))
      }
    })
    .buffer_unordered(8);

  let mut i = 0;
  while let Some(b) = bodies.next().await {
    let (v, embeddings) = b?;
    let points = embeddings
      .iter()
      .map(|e| {
        let e: Embedding = serde_json::from_str(e).unwrap();

        let mut payload = Payload::new();
        payload.insert("id", v.id as i64);

        i += 1;
        PointStruct::new(i, e.embedding, payload)
      })
      .collect();

    client.upsert_points(COLLECTION_NAME, points).await?;
    println!("Upserted verse {}", i);
  }

  println!("Exporting vector...");
  _export_vector().await?;
  println!("Exported");

  Ok(())
}

async fn embed(text: impl AsRef<str>, client: &Client) -> Result<String> {
  let url = format!("http://localhost:8000/embed?q={}", text.as_ref());
  let resp = client.get(url).send().await?;
  Ok(resp.text().await?)
}

#[tokio::main]
pub async fn export_vector() -> Result<()> {
  _export_vector().await
}

async fn _export_vector() -> Result<()> {
  let outfile = "qdrant.tar";
  let _ = std::fs::remove_dir_all(outfile);
  let _ = std::fs::remove_file(outfile);

  let client = QdrantClient::new(None).await?;
  client.create_snapshot(COLLECTION_NAME).await?;
  client
    .download_snapshot(outfile, COLLECTION_NAME, None, None)
    .await?;

  Ok(())
}
