use anyhow::Result;
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use once_cell::sync::Lazy;
use tokio_postgres::{Client, NoTls};

const DB: &'static str = "stb";

pub static POOL: Lazy<Pool> = Lazy::new(|| {
  let mut cfg = Config::new();
  cfg.dbname = Some(DB.into());
  cfg.manager = Some(ManagerConfig {
    recycling_method: RecyclingMethod::Fast,
  });
  cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
});

pub async fn connect(db: Option<&str>) -> Result<Client> {
  let mut uri = "postgresql://postgres:postgres@localhost/".to_owned();
  if let Some(db) = db {
    uri.push_str(db);
  }
  let (client, connection) = tokio_postgres::connect(&uri, NoTls).await?;

  // This task will die when the client is dropped
  tokio::spawn(async move {
    if let Err(e) = connection.await {
      eprintln!("connection error: {}", e);
    }
  });

  Ok(client)
}
