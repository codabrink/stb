use anyhow::Result;
use serde::Serialize;
use tokio_postgres::{GenericClient, Row};

#[derive(Serialize)]
pub struct Embedding {
  pub id: i32,
  pub verse_id: i32,
  pub book_order: i32,
  pub embedding: Vec<f32>,
  pub model: Model,
}

impl Embedding {
  async fn insert(&self, client: impl GenericClient) -> Result<()> {
    client
      .execute(
        "INSERT INTO embeddings (verse_id, book_order, embedding, model) VALUES ($1, $2, $3, $4)",
        &[
          &self.verse_id,
          &self.book_order,
          &self.embedding,
          &(self.model as i32),
        ],
      )
      .await?;

    Ok(())
  }
}

#[repr(i32)]
#[derive(Serialize, Copy, Clone)]
pub enum Model {
  BERT = 0,
  JINA = 1,
}

impl From<i32> for Model {
  fn from(value: i32) -> Self {
    match value {
      0 => Model::BERT,
      1 => Model::JINA,
      _ => unimplemented!(),
    }
  }
}

impl<'a> From<&'a Row> for Embedding {
  #[inline]
  fn from(row: &'a Row) -> Self {
    Self {
      id: row.get("id"),
      verse_id: row.get("verse_id"),
      book_order: row.get("book_order"),
      embedding: row.get("embedding"),
      model: row.get::<'a, &str, i32>("model").into(),
    }
  }
}
