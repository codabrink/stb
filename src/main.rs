#[macro_use]
extern crate rocket;

use std::time::Duration;

use anyhow::Result;
// use clap::Parser;
use http::rocket;
use tokio_postgres::{Error, NoTls};

mod args;
mod db;
mod http;
mod init;
mod model;
mod search;

#[rocket::main]
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

  rocket().await?;
  Ok(())
}
