use clap::Parser;
use std::thread;
use std::time::Duration;

mod args;
mod http;

fn main() {
  let args = args::Args::parse();

  match args.cmd.as_deref() {
    Some("init") => {
      // idk.. init
    }
    _ => {
      http::boot_server().expect("Problem with the web server");
    }
  }

  loop {
    // do more later
    thread::sleep(Duration::from_secs(1))
  }
}
