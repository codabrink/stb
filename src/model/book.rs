use crate::init::pg;
use anyhow::Result;
use serde::Serialize;
use tokio_postgres::Row;

#[derive(Serialize)]
pub struct Book {
  pub id: u32,
  pub slug: String,
  pub name: String,
  pub chapters: u32,
  pub order: u32,
}

impl Book {
  pub async fn all() -> Result<Vec<Self>> {
    Ok(
      pg()
        .await?
        .query("SELECT * FROM BOOKS", &[])
        .await?
        .into_iter()
        .map(Book::parse_row)
        .collect(),
    )
  }

  pub async fn query(slug: &str) -> Result<Self> {
    let row = pg()
      .await?
      .query_one(
        "SELECT * FROM books WHERE slug = (?1) ORDER BY ord",
        &[&slug],
      )
      .await?;

    Ok(Book::parse_row(row))
  }

  fn parse_row(row: Row) -> Book {
    Book {
      id: row.get("id"),
      slug: row.get("slug"),
      name: row.get("name"),
      chapters: row.get("chapters"),
      order: row.get("ord"),
    }
  }
}
