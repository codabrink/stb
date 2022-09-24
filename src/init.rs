use crate::prelude::*;

use futures::{stream, StreamExt};
use glob::glob;
use lazy_static::lazy_static;
use qdrant_client::{
  prelude::*,
  qdrant::{vectors_config, Distance, VectorParams, VectorsConfig},
};
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
      book_slug  TEXT NOT NULL,
      book       TEXT NOT NULL,
      book_order INTEGER NOT NULL,
      chapter    INTEGER NOT NULL,
      verse      INTEGER NOT NULL,
      content    TEXT NOT NULL
    );
    CREATE INDEX idx_verses ON verses (book_slug, book_order, chapter, verse);
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
        "INSERT INTO verses (book, book_order, book_slug, chapter, verse, content) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![book, order, slug, chapter, verse, line],
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

pub struct Embeddings {
  verse: u64,
  chapter: u64,
  slug: String,
  embeddings: Vec<Embedding>,
}

#[tokio::main]
pub async fn eh() -> Result<()> {
  let conn = Connection::open(SQLITE_DB)?;
  let embed_conn = Connection::open(EMBEDDING_DB)?;

  let mut stmt = embed_conn.prepare("SELECT id, verse_id FROM embeddings")?;
  let embedding_iter = stmt.query_map([], |row| {
    let id: u64 = row.get(0)?;
    let verse_id: u64 = row.get(1)?;
    Ok((id, verse_id))
  })?;

  for row in embedding_iter {
    if let Ok((id, verse_id)) = row {
      let (verse, chapter, book_slug) = conn.query_row(
        "SELECT verse, chapter, book_slug FROM verses WHERE id = ?1",
        params![verse_id],
        |row| {
          let verse: u64 = row.get(0)?;
          let chapter: u64 = row.get(1)?;
          let book_slug: String = row.get(2)?;
          Ok((verse, chapter, book_slug))
        },
      )?;

      embed_conn.execute(
        "UPDATE embeddings SET verse = ?1, chapter = ?2, slug = ?3 WHERE id = ?4",
        params![verse, chapter, book_slug, id],
      )?;
    }
  }

  Ok(())
}

#[tokio::main]
pub async fn collect_embeddings() -> Result<()> {
  if Path::new(EMBEDDING_DB).exists() {
    println!("Embedding datbase already exists. Delete file if you want to reset.");
    return Ok(());
  }

  let conn = Connection::open(SQLITE_DB)?;
  let embed_conn = Connection::open(EMBEDDING_DB)?;
  embed_conn.execute(
    "
    CREATE TABLE embeddings (
      id         INTEGER PRIMARY KEY,
      verse_id   INTEGER NOT NULL,
      verse      INTEGER NOT NULL,
      chapter    INTEGER NOT NULL,
      slug       TEXT NOT NULL,
      embeddings TEXT NOT NULL
    );
    CREATE INDEX idx_embeddings ON embeddings (verse_id);
  ",
    [],
  )?;

  let mut stmt =
    conn.prepare("SELECT id, book, book_slug, chapter, verse, content, book_order FROM verses")?;

  let verse_iter = stmt.query_map([], |row| {
    Ok(Verse {
      id: row.get(0)?,
      book: row.get(1)?,
      book_slug: row.get(2)?,
      chapter: row.get(3)?,
      verse: row.get(4)?,
      content: row.get(5)?,
      book_order: row.get(6)?,
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

  while let Some(b) = bodies.next().await {
    let (v, embeddings) = b?;

    embed_conn.execute(
      "INSERT INTO embeddings (verse_id, embeddings) VALUES (?1, ?2)",
      params![v.id, serde_json::to_string(&embeddings)?],
    )?;
    println!("Stored embedding for {}", v);
  }

  Ok(())
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
      vectors_config: Some(VectorsConfig {
        config: Some(vectors_config::Config::Params(VectorParams {
          size: 768,
          distance: Distance::Dot as i32,
        })),
      }),
      ..Default::default()
    })
    .await?;

  let conn = Connection::open(SQLITE_DB)?;
  let embed_conn = Connection::open(EMBEDDING_DB)?;

  let mut stmt = embed_conn.prepare("SELECT slug, verse, chapter, embeddings FROM embeddings")?;
  let embed_iter = stmt.query_map([], |row| {
    let embeddings_string: String = row.get(3)?;
    let embeddings: Vec<String> = serde_json::from_str(&embeddings_string).unwrap();

    Ok(Embeddings {
      slug: row.get(0)?,
      verse: row.get(1)?,
      chapter: row.get(2)?,
      embeddings: embeddings
        .iter()
        .map(|e| serde_json::from_str(e).unwrap())
        .collect(),
    })
  })?;

  let mut i = 0;
  for verse in embed_iter {
    let ve = verse?;

    let verse = conn.query_row(
      "SELECT * FROM verses WHERE book_slug = ?1 AND chapter = ?2 AND verse = ?3",
      params![ve.slug, ve.chapter, ve.verse],
      Verse::parse_row,
    )?;

    let mut points = Vec::with_capacity(ve.embeddings.len());
    for embedding in ve.embeddings {
      let mut payload = Payload::new();
      payload.insert("id", verse.id as i64);
      payload.insert(
        "is_apocrypha",
        verse.book_order > 38 && verse.book_order < 54,
      );

      i += 1;
      points.push(PointStruct::new(i, embedding.embedding, payload));
    }

    client.upsert_points(COLLECTION_NAME, points).await?;
    println!("Upserted {}", verse);
  }

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
