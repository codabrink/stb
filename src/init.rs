use anyhow::Result;
use glob::glob;
use lazy_static::lazy_static;
use postgres::{Client, NoTls};
use regex::Regex;
use rust_bert::pipelines::sentence_embeddings::{
  SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use serde::Deserialize;

use crate::model::Verse;

lazy_static! {
  static ref SPLIT_REGEX: Regex = Regex::new(r",|\.").unwrap();
}

pub fn reset_db() -> Result<()> {
  let mut client = Client::connect("host=localhost user=postgres", NoTls)?;
  let _ = client.execute("DROP DATABASE stb;", &[]);
  client.execute("CREATE DATABASE stb;", &[])?;
  pg()?.execute("CREATE EXTENSION vector;", &[])?;

  Ok(())
}

pub fn pg() -> Result<Client> {
  Ok(Client::connect(
    "postgresql://postgres:postgres@localhost/stb",
    NoTls,
  )?)
}

pub fn rebuild_sql() -> Result<()> {
  reset_db()?;
  let mut client = pg()?;

  client.batch_execute(
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
      embedding  vector(768)
    );
    CREATE INDEX idx_embeddings ON embeddings (verse_id);
    ",
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
    let chapter: i32 = chapter_regex.captures(&lines[1]).unwrap()[1].parse()?;

    {
      let count: i64 = client
        .query_one("SELECT COUNT(*) FROM books WHERE slug = ($1)", &[&slug])?
        .get(0);
      if count == 0 {
        client.execute(
          "INSERT INTO books (slug, name, chapters, ord) VALUES ($1, $2, $3, $4);",
          &[&slug, &book, &1, &order],
        )?;
        order += 1;
      } else {
        client.execute(
          "UPDATE books SET chapters = chapters + 1 WHERE slug = ($1)",
          &[&slug],
        )?;
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

pub fn collect_embeddings() -> Result<()> {
  let mut client = pg()?;
  let model = SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllDistilrobertaV1)
    .create_model()?;

  const LIMIT: usize = 1000;
  let mut offset = 0;

  loop {
    let rows = client.query(
      &format!("SELECT id, book, book_slug, chapter, verse, content, book_order FROM verses LIMIT {LIMIT} OFFSET {offset}"),
      &[],
    )?;
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

      let mut fragments = Vec::new();

      fragments.push(verse.content.clone());

      let splits: Vec<String> = SPLIT_REGEX
        .split(&verse.content)
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
          // TODO: optimize
          fragments.push(split.trim().to_owned());
        }
      }

      for i in 1..(splits.len() - 1) {
        let subverse = splits[i..]
          .iter()
          .map(|s| s.trim())
          .collect::<Vec<&str>>()
          .join(" ");

        println!("SUBVERSE: {}", &subverse);
        fragments.push(subverse);
      }

      println!(
        "Gathered embedding for {} Chapter {} Verse {} ({} splits)",
        verse.book,
        verse.chapter,
        verse.verse,
        &splits.len()
      );

      let embeddings = embed(fragments, &model)?;

      for embedding in embeddings {
        let embedding = serde_json::to_string(&embedding)?;
        client.execute(
          &format!("INSERT INTO embeddings (verse_id, book_order, embedding) VALUES ($1, $2, '{embedding}')"),
          &[&verse.id, &verse.book_order],
        )?;
      }

      println!("Stored embedding for {}", verse);
    }

    offset += LIMIT;
  }

  Ok(())
}

fn embed(texts: Vec<String>, model: &SentenceEmbeddingsModel) -> Result<Vec<Vec<f32>>> {
  Ok(model.encode(&texts)?)
}
