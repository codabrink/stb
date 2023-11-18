use anyhow::{Error as E, Result};
use candle_core::{Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::jina_bert::{BertModel, Config};
use crossbeam_channel::{unbounded, Sender};
use std::sync::Once;
use tokio::sync::oneshot::{self, Sender as OSSender};

use crate::model::Verse;

type EmbedResult = Vec<Vec<f32>>;
static mut TX: Option<Sender<(Vec<String>, OSSender<EmbedResult>)>> = None;
static INIT_MODEL: Once = Once::new();

fn embed_tx() -> Sender<(Vec<String>, OSSender<EmbedResult>)> {
  unsafe {
    INIT_MODEL.call_once(|| {
      let (tx, rx) = unbounded::<(Vec<String>, OSSender<EmbedResult>)>();
      std::thread::spawn(move || -> Result<()> {
        use hf_hub::{api::sync::Api, Repo, RepoType};

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
        let mut tokenizer =
          tokenizers::Tokenizer::from_file(tokenizer).map_err(anyhow::Error::msg)?;
        let vb = unsafe {
          VarBuilder::from_mmaped_safetensors(&[model], candle_core::DType::F32, &device)?
        };
        let model = BertModel::new(vb, &config)?;

        let tokenizer = tokenizer
          .with_padding(None)
          .with_truncation(None)
          .map_err(anyhow::Error::msg)?;

        if let Some(pp) = tokenizer.get_padding_mut() {
          pp.strategy = tokenizers::PaddingStrategy::BatchLongest;
        } else {
          let pp = tokenizers::PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
          };
          tokenizer.with_padding(Some(pp));
        }

        for (sentence, tx) in rx {
          let tokens = tokenizer
            .encode_batch(vec![sentence], true)
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

          let (n_sentences, n_tokens, _hidden_size) = embeddings.dims3()?;
          let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;

          let mut result: EmbedResult = vec![];
          for i in 0..n_sentences {
            result.push(embeddings.get(i)?.to_vec1()?);
          }

          let _ = tx.send(result);
        }

        Ok(())
      });

      TX = Some(tx);
    });
    TX.as_ref().unwrap().clone()
  }
}

pub async fn embed(sentences: Vec<String>) -> Result<EmbedResult> {
  let (os_tx, rx) = oneshot::channel();
  let tx = embed_tx();
  tx.send((sentences, os_tx))?;
  Ok(rx.await?)
}

pub async fn search(
  query: impl ToString,
  limit: usize,
  include_apocrypha: bool,
) -> Result<Vec<Verse>> {
  let client = crate::db::POOL.get().await?;
  let embedding = serde_json::to_string(&embed(vec![query.to_string()]).await?[0])?;

  let rows: Vec<Verse> = client
    .query(
      &format!(
        "
      WITH embeddings AS (
        SELECT verse_id, embedding <=> '{embedding}' AS distance
        FROM embeddings WHERE model = 1 ORDER BY distance LIMIT {limit}
      )

      SELECT *, embeddings.distance
      FROM verses JOIN embeddings ON verses.id = embeddings.verse_id
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
