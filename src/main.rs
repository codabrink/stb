#[macro_use]
extern crate rocket;

use anyhow::Result;
// use clap::Parser;
use http::rocket;

mod args;
mod http;
mod init;
mod model;
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

  rocket().await?;
  Ok(())
}
