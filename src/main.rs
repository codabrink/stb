#[macro_use]
extern crate rocket;

use clap::Parser;

mod args;
mod http;
mod init;
mod model;
mod prelude;
mod search;

static mut QDRANT_HOST: String = String::new();
static mut EMBEDDER_HOST: String = String::new();

pub fn qdrant_host() -> &'static str {
  unsafe { &QDRANT_HOST }
}
pub fn embedder_host() -> &'static str {
  unsafe { &EMBEDDER_HOST }
}

fn main() {
  unsafe {
    QDRANT_HOST = match std::env::var("QDRANT_HOST") {
      Ok(host) => host,
      _ => String::from("localhost"),
    };
    EMBEDDER_HOST = match std::env::var("EMBEDDER_HOST") {
      Ok(host) => host,
      _ => String::from("localhost"),
    }
  }

  let args = args::Args::parse();
  directories();

  if args.rebuild {
    init::rebuild_sql().expect("Problem rebuilding sql");
    init::rebuild_vector().expect("Problem rebuilding vector");
  }
  if args.rebuild_sql {
    init::rebuild_sql().expect("could not init");
  }
  if args.rebuild_vector {
    init::rebuild_vector().expect("could not init");
  }
  if args.export_vector {
    init::export_vector().expect("Could not export from qdrant");
  }

  if let Some(query) = &args.search {
    search::search_blocking(query, 10).expect("Could not search");
  }
  if args.server {
    println!("Running http server...");
    http::rocket().expect("Issue running http server");
  }
}

fn directories() {
  let _ = std::fs::create_dir_all("static/sass");
  let _ = std::fs::create_dir_all("static/css");
}
