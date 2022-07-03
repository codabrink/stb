use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
  #[clap(value_parser)]
  pub cmd: Option<String>,
}
