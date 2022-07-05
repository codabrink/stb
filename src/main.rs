use clap::Parser;
use std::thread;
use std::time::Duration;

mod args;
mod http;
mod init;
mod search;

fn main() {
  let args = args::Args::parse();

  if args.init_sql {
    init::build_sql().expect("could not init");
  }
  if args.init_vector {
    init::build_vector().expect("could not init");
  }

  if let Some(query) = &args.search {
    search::search(query).expect("Could not search");
  }
  if args.server {
    http::boot_server().expect("Problem with the web server");

    loop {
      // do more later
      thread::sleep(Duration::from_secs(1))
    }
  }
  println!("{:?}", args.search);
}
