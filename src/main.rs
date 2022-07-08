use clap::Parser;
use std::thread;
use std::time::Duration;

mod args;
mod http;
mod init;
mod search;

fn main() {
  let args = args::Args::parse();

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

  if let Some(query) = &args.search {
    search::search_blocking(query, 10).expect("Could not search");
  }
  if args.server {
    http::boot_server().expect("Problem with the web server");

    loop {
      // do more later
      thread::sleep(Duration::from_secs(1))
    }
  }
}
