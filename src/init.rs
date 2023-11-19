use crate::candle::embed;
use crate::model::embedding::Embedding;
use crate::model::{Book, Quote, Verse};
use crate::{db::POOL, model::embedding::Model};
use anyhow::Result;
use deadpool_postgres::Object;
use glob::glob;
use once_cell::sync::Lazy;
use regex::Regex;
#[cfg(feature = "rust_bert")]
use rust_bert::pipelines::sentence_embeddings::{
  SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use std::collections::HashSet;
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
        part       INTEGER NOT NULL,
        content    TEXT NOT NULL
      );
      CREATE INDEX idx_verses ON verses (book_slug, book_order, chapter, verse, part);

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

      CREATE TABLE characters (
        id   SERIAL PRIMARY KEY,
        name TEXT NOT NULL
      );
      CREATE UNIQUE INDEX idx_charachter_name ON characters (name);

      CREATE TABLE quotes (
        id SERIAL     PRIMARY KEY,
        from_verse_id INTEGER NOT NULL,
        to_verse_id   INTEGER NOT NULL,
        character_id  INTEGER
      );
      CREATE UNIQUE INDEX idx_quotes_from_to_verse ON quotes (from_verse_id, to_verse_id);
      CREATE INDEX idx_quote_character ON quotes (character_id);
      ",
    )
    .await?;

  Ok(())
}

pub async fn pg() -> Result<Object> {
  Ok(POOL.get().await?)
}

pub const LQ: char = '“';
pub const RQ: char = '”';
static PUNCTUATION: Lazy<Regex> = Lazy::new(|| Regex::new(r",|\.").unwrap());

pub async fn rebuild_sql() -> Result<()> {
  reset_db().await?;
  let mut client = pg().await?;

  let chapter_regex = Regex::new(r"Chapter\s(\d+)\.")?;

  let mut order = 0;

  for entry in glob("bible/eng-web_*.txt").expect("Failed to read Bible directory") {
    let entry = entry?;
    let file_name = entry.file_name().unwrap().to_string_lossy().to_string();

    let slug = file_name.split('_').nth(2).unwrap();

    let text_file = std::fs::read_to_string(&entry)?;
    let lines: Vec<&str> = text_file.split('\n').collect();
    let words: Vec<Vec<String>> = lines
      .iter()
      .map(|v| {
        v.split(' ')
          .map(|word| PUNCTUATION.replace(word, "").to_lowercase().to_string())
          .collect()
      })
      .collect();

    // chopping the first 3 bytes is to trim the BOM
    let book = &lines[0][..lines[0].len() - 1][3..];
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

    let transaction = client.transaction().await?;

    let mut verse = 1;
    for line in &lines[2..] {
      if line.trim().is_empty() {
        continue;
      }

      let mut parts = Verse::to_parts(line);

      for (i, part) in parts.iter().enumerate() {
        transaction.execute(
          "INSERT INTO verses (book, book_order, book_slug, chapter, verse, content, part) VALUES ($1, $2, $3, $4, $5, $6, $7)",
          &[&book, &order, &slug, &chapter, &verse, &part, &((i + 1) as i32)],
        ).await?;
      }

      verse += 1;
    }

    transaction.commit().await?;
  }

  println!("Building quotes...");
  build_quotes().await?;

  Ok(())
}

pub async fn build_quotes() -> Result<()> {
  let books = Book::all().await?;

  let mut quotes = vec![];
  let mut characters = HashSet::new();
  let mut client = pg().await?;
  let transaction = client.transaction().await?;

  let mut quote = None;
  for book in books {
    for chapter in 1..book.chapters {
      let verse_parts = Verse::query(&book.slug, chapter, None).await?;
      for (i, verse_part) in verse_parts.iter().enumerate() {
        if verse_part.content.starts_with(LQ) {
          quote = Some(Quote {
            id: 0,
            from_verse_id: verse_part.id,
            to_verse_id: 0,
            charcter_id: None,
          });
        }
        if verse_part.content.ends_with(RQ) {
          if let Some(mut q) = quote.take() {
            q.to_verse_id = verse_part.id;
            q.insert(&transaction).await?;
            let character = find_character(&verse_parts[i.saturating_sub(6)..=i]).await?;
            if let Some(c) = character {
              characters.insert(c);
            }
            quotes.push(q);
          }
        }
      }
    }
  }

  transaction.commit().await?;

  // println!("{} quotes", quotes.len());
  dbg!(&characters);
  println!("{} characters", characters.len());

  Ok(())
}

const INITIATIVE: &'static [&'static str] = &[" said", " called"];

async fn find_character(verses: &[Verse]) -> Result<Option<String>> {
  for i in (0..verses.len()).rev() {
    for init in INITIATIVE {
      let verse = &verses[i];
      if verse.content.contains(init) {
        let words: Vec<&str> = verse.content.split(' ').collect();
        let init = init.trim();

        // prone to crashing
        let i = words.iter().position(|w| w.starts_with(init)).unwrap();
        let character = words[i - 1];

        return Ok(Some(character.to_owned()));
      }
    }
  }

  Ok(None)
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
  transaction.commit().await?;

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
