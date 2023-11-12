use crate::init::pg;
use anyhow::Result;
use serde::Serialize;
use tokio_postgres::Row;

#[derive(Serialize)]
pub struct Book {
  pub id: i32,
  pub slug: String,
  pub name: String,
  pub chapters: i32,
  pub order: i32,
}

impl Book {
  pub async fn all() -> Result<Vec<Self>> {
    Ok(
      pg()
        .await?
        .query("SELECT * FROM BOOKS", &[])
        .await?
        .into_iter()
        .map(Book::from)
        .collect(),
    )
  }

  pub async fn fetch(slug: &str) -> Result<Self> {
    let row = pg()
      .await?
      .query_one(
        "SELECT * FROM books WHERE slug = (?1) ORDER BY ord",
        &[&slug],
      )
      .await?;

    Ok(row.into())
  }
}

impl From<&Row> for Book {
  #[inline]
  fn from(row: &Row) -> Self {
    Self {
      id: row.get("id"),
      slug: row.get("slug"),
      name: row.get("name"),
      chapters: row.get("chapters"),
      order: row.get("ord"),
    }
  }
}

impl From<Row> for Book {
  fn from(row: Row) -> Self {
    (&row).into()
  }
}
