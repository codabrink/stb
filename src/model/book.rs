use crate::init::pg;
use anyhow::Result;
use postgres::Row;
use serde::Serialize;

#[derive(Serialize)]
pub struct Book {
  pub id: u32,
  pub slug: String,
  pub name: String,
  pub chapters: u32,
  pub order: u32,
}

impl Book {
  pub fn all() -> Result<Vec<Self>> {
    Ok(
      pg()?
        .query("SELECT * FROM BOOKS", &[])?
        .into_iter()
        .map(Book::parse_row)
        .collect(),
    )
  }

  pub fn query(slug: &str) -> Result<Self> {
    let row = pg()?.query_one(
      "SELECT * FROM books WHERE slug = (?1) ORDER BY ord",
      &[&slug],
    )?;

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
