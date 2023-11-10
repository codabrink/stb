use crate::{init::pg, model::Verse};
use anyhow::Result;
use crossbeam_channel::{unbounded, Sender};
use rust_bert::pipelines::sentence_embeddings::{
  SentenceEmbeddingsBuilder, SentenceEmbeddingsModelType,
};
use std::sync::Once;
use tokio::sync::oneshot::{self, Sender as OSSender};
use tokio_postgres::NoTls;

static mut TX: Option<Sender<(String, OSSender<Vec<f32>>)>> = None;

static INIT_MODEL: Once = Once::new();

// Create a channel to the worker thread for an embedding request
fn embed_tx() -> Sender<(String, OSSender<Vec<f32>>)> {
  unsafe {
    INIT_MODEL.call_once(|| {
      let (tx, rx) = unbounded::<(String, OSSender<Vec<f32>>)>();
      std::thread::spawn(move || {
        let model =
          SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllDistilrobertaV1)
            .create_model()
            .unwrap();
        for (string, tx) in rx {
          let mut results = model.encode(&[string]).unwrap();
          let _ = tx.send(results.pop().unwrap());
        }
      });

      TX = Some(tx);
    });
    TX.as_ref().unwrap().clone()
  }
}

async fn embed(string: String) -> Result<Vec<f32>> {
  let (os_tx, rx) = oneshot::channel();
  let tx = embed_tx();
  tx.send((string.to_string(), os_tx))?;
  Ok(rx.await?)
}

pub async fn search(
  query: impl ToString,
  limit: usize,
  include_apocrypha: bool,
) -> Result<Vec<Verse>> {
  let client = crate::db::POOL.get().await?;

  let embedding = serde_json::to_string(&embed(query.to_string()).await?)?;

  let rows: Vec<Verse> = client
    .query(
      &format!(
        "
      WITH embeddings AS (
        SELECT verse_id, embedding <-> '{embedding}' AS distance FROM embeddings ORDER BY distance LIMIT {limit}
      )

      SELECT *, embeddings.distance
      FROM verses JOIN embeddings ON verses.id = embeddings.verse_id
      ORDER BY embeddings.distance
      "
      ),
      &[],
    )
    .await?
    .into_iter()
    .map(Verse::from)
    .collect();

  Ok(rows)
}
