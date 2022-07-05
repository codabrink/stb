use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
  #[clap(long)]
  pub init_sql: bool,

  #[clap(long)]
  pub init_vector: bool,

  #[clap(long, short)]
  pub server: bool,

  #[clap(long)]
  pub search: Option<String>,

  #[clap(value_parser)]
  pub cmd: Option<String>,
}
