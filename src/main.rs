use anyhow::Result;
// use clap::Parser;

use tokio_postgres::{Error, NoTls};

mod args;
mod candle;
mod db;
mod http;
mod init;
mod model;
#[cfg(feature = "rust_bert")]
mod search;

#[tokio::main]
async fn main() -> Result<()> {
  // let args = args::Args::parse();

  // init::rebuild_sql()?;
  // init::collect_embeddings().expect("Problem rebuilding vector");

  // let a = search_blocking("Where is the peace of God?", 0, false)?;
  // for b in a {
  // println!("{}", b);
  // }

  // init::jina_embeddings().await?;

  // init::summary().await?;

  http::serve().await?;
  Ok(())
}
