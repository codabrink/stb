use crate::db::POOL;
use crate::model::Verse;
use anyhow::{Error as E, Result};
use candle_core::{Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::jina_bert::{BertModel, Config};
use deadpool_postgres::Object;
use glob::glob;
use once_cell::sync::Lazy;
use regex::Regex;
use rust_bert::pipelines::sentence_embeddings::{
  SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
};
use serde::Deserialize;

static SPLIT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r",|\.").unwrap());

pub async fn reset_db() -> Result<()> {
  let client = crate::db::connect(None).await?;
  let _ = client.execute("DROP DATABASE stb;", &[]).await;
  client.execute("CREATE DATABASE stb;", &[]).await?;

  crate::db::connect(Some("stb"))
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

pub async fn jina_embeddings() -> Result<()> {
  use hf_hub::{api::sync::Api, Repo, RepoType};

  let mut client = pg().await?;

  let model = Api::new()?
    .repo(Repo::new(
      "jinaai/jina-embeddings-v2-base-en".to_string(),
      RepoType::Model,
    ))
    .get("model.safetensors")?;

  let tokenizer = Api::new()?
    .repo(Repo::new(
      "sentence-transformers/all-MiniLM-L6-v2".to_string(),
      RepoType::Model,
    ))
    .get("tokenizer.json")?;

  let device = Device::Cpu;
  let config = Config::v2_base();
  let mut tokenizer = tokenizers::Tokenizer::from_file(tokenizer).map_err(anyhow::Error::msg)?;
  let vb =
    unsafe { VarBuilder::from_mmaped_safetensors(&[model], candle_core::DType::F32, &device)? };
  let model = BertModel::new(vb, &config)?;

  let tokenizer = tokenizer
    .with_padding(None)
    .with_truncation(None)
    .map_err(anyhow::Error::msg)?;

  const LIMIT: usize = 100;
  let fetch_statement = format!("FETCH {LIMIT} FROM curs;");

  let mut transaction = client.transaction().await?;
  transaction
    .batch_execute("DECLARE curs CURSOR FOR SELECT * FROM verses;")
    .await?;

  loop {
    let rows = transaction.query(&fetch_statement, &[]).await?;

    for row in &rows {
      let verse = Verse::from(row);
      let fragments = shatter_verse(&verse)?;

      for fragments in fragments.chunks(10) {
        if let Some(pp) = tokenizer.get_padding_mut() {
          pp.strategy = tokenizers::PaddingStrategy::BatchLongest;
        } else {
          let pp = tokenizers::PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
          };
          tokenizer.with_padding(Some(pp));
        }

        let tokens = tokenizer
          .encode_batch(fragments.into(), true)
          .map_err(E::msg)?;
        let token_ids = tokens
          .iter()
          .map(|tokens| {
            let tokens = tokens.get_ids().to_vec();
            Tensor::new(tokens.as_slice(), &device)
          })
          .collect::<candle_core::Result<Vec<_>>>()?;
        let token_ids = Tensor::stack(&token_ids, 0)?;

        let embeddings = model.forward(&token_ids)?;

        let (n_fragments, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;

        for i in 0..n_fragments {
          let embedding = embeddings.get(i)?;
          let embedding: Vec<f32> = embedding.to_vec1()?;
          let embedding = serde_json::to_string(&embedding)?;

          transaction.execute(
          &format!("INSERT INTO embeddings (verse_id, book_order, embedding, model) VALUES ($1, $2, '{embedding}', 1)"),
          &[&verse.id, &verse.book_order],
        ).await?;

          println!("{embedding:?}");
        }
      }
    }

    if rows.len() < LIMIT {
      break;
    }
  }

  Ok(())
}

fn shatter_verse(verse: &Verse) -> Result<Vec<String>> {
  let mut fragments = vec![verse.content.clone()];

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
    "Shattered verse {} {}:{} ({} splits)",
    verse.book,
    verse.chapter,
    verse.verse,
    &splits.len()
  );

  Ok(fragments)
}

pub async fn collect_embeddings() -> Result<()> {
  let mut client = pg().await?;
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

      let fragments = shatter_verse(&verse)?;
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

fn embed(texts: Vec<String>, model: &SentenceEmbeddingsModel) -> Result<Vec<Vec<f32>>> {
  Ok(model.encode(&texts)?)
}
