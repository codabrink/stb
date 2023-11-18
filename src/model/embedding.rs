use anyhow::Result;
use deadpool_postgres::GenericClient;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

#[derive(Serialize, Deserialize)]
pub struct Embedding {
  pub id: i32,
  pub verse_id: i32,
  pub book_order: i32,
  pub embedding: Vec<f32>,
  pub model: Model,
}

impl Embedding {
  pub async fn insert(&self, client: &impl GenericClient) -> Result<()> {
    let embedding = serde_json::to_string(&self.embedding)?;

    client
      .execute(
        &format!("INSERT INTO embeddings (verse_id, book_order, embedding, model) VALUES ($1, $2, '{embedding}', $3)"),
        &[
          &self.verse_id,
          &self.book_order,
          &(self.model as i32),
        ],
      )
      .await?;

    Ok(())
  }
}

#[repr(i32)]
#[derive(Serialize, Deserialize, Copy, Clone)]
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
