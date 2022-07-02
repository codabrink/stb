use std::thread;
use std::time::Duration;

mod http;

fn main() {
  http::boot_server().expect("Problem with the web server");

  loop {
    // do more later
    thread::sleep(Duration::from_secs(1))
  }
}
