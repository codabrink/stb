use crate::candle::embed;
use crate::model::embedding::Embedding;
use crate::model::Verse;
use crate::{db::POOL, model::embedding::Model};
use anyhow::{Error as E, Result};
use candle_core::{Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::jina_bert::{BertModel, Config};
use deadpool_postgres::Object;
use glob::glob;
use regex::Regex;
#[cfg(feature = "rust_bert")]
use rust_bert::pipelines::sentence_embeddings::{
  SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use std::fs::OpenOptions;
use std::io::prelude::*;
use tokio::fs::File;
use tokio::io::{self, AsyncBufReadExt};

pub async fn reset_db() -> Result<()> {
  let client = crate::db::connect(Some("")).await?;
  let _ = client.execute("DROP DATABASE stb;", &[]).await;
  client.execute("CREATE DATABASE stb;", &[]).await?;

  crate::db::connect(None)
    .await?
    .batch_execute("CREATE EXTENSION vector;")
    .await?;

  Ok(())
}

pub async fn pg() -> Result<Object> {
  Ok(POOL.get().await?)
}

pub async fn rebuild_sql() -> Result<()> {
  reset_db().await?;
  let client = pg().await?;

  client
    .batch_execute(
      "
    CREATE TABLE verses (
      id         SERIAL PRIMARY KEY,
      book_slug  TEXT NOT NULL,
      book       TEXT NOT NULL,
      book_order INTEGER NOT NULL,
      chapter    INTEGER NOT NULL,
      verse      INTEGER NOT NULL,
      content    TEXT NOT NULL
    );
    CREATE INDEX idx_verses ON verses (book_slug, book_order, chapter, verse);

    CREATE TABLE books (
      id       SERIAL PRIMARY KEY,
      slug     TEXT NOT NULL,
      name     TEXT NOT NULL,
      chapters INTEGER NOT NULL,
      ord      INTEGER NOT NULL
    );
    CREATE INDEX idx_books ON books (ord, slug);

    CREATE TABLE embeddings (
      id         SERIAL PRIMARY KEY,
      verse_id   INTEGER NOT NULL,
      book_order INTEGER NOT NULL,
      embedding  vector(768),
      model      INTEGER NOT NULL
    );
    CREATE INDEX idx_embeddings ON embeddings (verse_id);
    CREATE INDEX idx_model ON embeddings (model);
    CREATE UNIQUE INDEX idx_unique ON embeddings (model, verse_id, book_order);
    ",
    )
    .await?;

  let chapter_regex = Regex::new(r"Chapter\s(\d+)\.")?;

  let mut order = 0;

  for entry in glob("bible/eng-web_*.txt").expect("Failed to read Bible directory") {
    let entry = entry?;
    let file_name = entry.file_name().unwrap().to_string_lossy().to_string();

    let slug = file_name.split('_').nth(2).unwrap();

    let text_file = std::fs::read_to_string(&entry)?;
    let lines: Vec<&str> = text_file.split('\n').collect();

    let book = &lines[0][..lines[0].len() - 1];
    let chapter: i32 = chapter_regex.captures(&lines[1]).unwrap()[1].parse()?;

    {
      let count: i64 = client
        .query_one("SELECT COUNT(*) FROM books WHERE slug = ($1)", &[&slug])
        .await?
        .get(0);
      if count == 0 {
        client
          .execute(
            "INSERT INTO books (slug, name, chapters, ord) VALUES ($1, $2, $3, $4);",
            &[&slug, &book, &1, &order],
          )
          .await?;
        order += 1;
      } else {
        client
          .execute(
            "UPDATE books SET chapters = chapters + 1 WHERE slug = ($1)",
            &[&slug],
          )
          .await?;
      }
    }

    println!(
      "Book: {book} ({slug}), Chapter: {chapter}, file name: {}",
      entry.file_name().unwrap().to_string_lossy()
    );

    let mut verse = 1;
    for line in &lines[2..] {
      if line.trim().is_empty() {
        continue;
      }

      client.execute(
        "INSERT INTO verses (book, book_order, book_slug, chapter, verse, content) VALUES ($1, $2, $3, $4, $5, $6)",
        &[&book, &order, &slug, &chapter, &verse, &line],
      ).await?;

      verse += 1;
    }
  }

  Ok(())
}

pub async fn jina_embeddings() -> Result<()> {
  let mut client = pg().await?;
  const LIMIT: usize = 100;
  let fetch_statement = format!("FETCH {LIMIT} FROM curs;");

  let transaction = client.transaction().await?;
  transaction
    .batch_execute("DECLARE curs CURSOR FOR SELECT * FROM verses;")
    .await?;

  const EMBEDDINGS_CACHE: &'static str = "jina_embeddings.json";

  let _ = std::fs::remove_file(EMBEDDINGS_CACHE);
  let mut embedding_file = OpenOptions::new()
    .write(true)
    .append(true)
    .create(true)
    .open(EMBEDDINGS_CACHE)?;

  loop {
    let rows = transaction.query(&fetch_statement, &[]).await?;

    for row in &rows {
      let verse = Verse::from(row);
      let fragments = verse.shatter()?;

      for fragments in fragments.chunks(10) {
        let embeddings = embed(fragments.to_vec()).await?;
        for embedding in embeddings {
          let embedding = Embedding {
            embedding,
            id: 0,
            verse_id: verse.id,
            book_order: verse.book_order,
            model: Model::JINA,
          };

          let embedding = serde_json::to_string(&embedding)?;
          writeln!(embedding_file, "{embedding}")?;
        }
      }
    }

    if rows.len() < LIMIT {
      break;
    }
  }

  drop(embedding_file);
  drop(transaction);

  let file = File::open("jina_embeddings.json").await?;
  let mut buffer = io::BufReader::new(file).lines();

  while let Some(line) = buffer.next_line().await? {
    let embedding: Embedding = serde_json::from_str(&line)?;
    embedding.insert(&client).await?;
  }

  Ok(())
}

#[cfg(feature = "rust_bert")]
pub async fn collect_embeddings() -> Result<()> {
  let client = pg().await?;
  let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllDistilrobertaV1)
    .create_model()?;

  const LIMIT: usize = 1000;
  let mut offset = 0;

  loop {
    let rows = client.query(
      &format!("SELECT id, book, book_slug, chapter, verse, content, book_order FROM verses LIMIT {LIMIT} OFFSET {offset}"),
      &[],
    ).await?;
    if rows.is_empty() {
      break;
    }

    for row in rows {
      let verse = Verse {
        id: row.get(0),
        book: row.get(1),
        book_slug: row.get(2),
        chapter: row.get(3),
        verse: row.get(4),
        content: row.get(5),
        book_order: row.get(6),
        distance: 0.,
      };

      let fragments = verse.shatter()?;
      let embeddings = embed(fragments, &model)?;

      for embedding in embeddings {
        let embedding = serde_json::to_string(&embedding)?;
        client.execute(
          &format!("INSERT INTO embeddings (verse_id, book_order, embedding) VALUES ($1, $2, '{embedding}')"),
          &[&verse.id, &verse.book_order],
        ).await?;
      }

      println!("Stored embedding for {}", verse);
    }

    offset += LIMIT;
  }

  Ok(())
}

#[cfg(feature = "rust_bert")]
fn embed(texts: Vec<String>, model: &SentenceEmbeddingsModel) -> Result<Vec<Vec<f32>>> {
  Ok(model.encode(&texts)?)
}
