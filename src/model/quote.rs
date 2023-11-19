use anyhow::Result;
use deadpool_postgres::GenericClient;

pub struct Quote {
  pub id: i32,
  pub from_verse_id: i32,
  pub to_verse_id: i32,
  pub charcter_id: Option<i32>,
}

impl Quote {
  pub async fn insert(&self, client: &impl GenericClient) -> Result<()> {
    client
      .execute(
        "INSERT INTO quotes (from_verse_id, to_verse_id, character_id) VALUES ($1, $2, $3)",
        &[&self.from_verse_id, &self.to_verse_id, &self.charcter_id],
      )
      .await?;

    Ok(())
  }
}
