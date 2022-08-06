pub use crate::prelude::*;

#[derive(Serialize)]
pub struct Book {
  pub id: u64,
  pub slug: String,
  pub name: String,
  pub chapters: u64,
  pub order: u64,
}

impl Book {
  pub fn all() -> Result<Vec<Self>> {
    let conn = Connection::open(SQLITE_DB)?;
    let mut stmt = conn.prepare("SELECT * FROM books")?;
    let rows = stmt.query_map([], Self::parse_row)?;
    Ok(rows.flat_map(|b| b).collect())
  }

  pub fn query(slug: &str) -> Result<Self> {
    let conn = Connection::open(SQLITE_DB)?;
    let query = "SELECT * FROM books WHERE slug = (?1) ORDER BY ord";
    Ok(conn.query_row(query, [slug], Book::parse_row)?)
  }

  fn parse_row(row: &Row) -> Result<Book, rusqlite::Error> {
    Ok(Book {
      id: row.get("id")?,
      slug: row.get("slug")?,
      name: row.get("name")?,
      chapters: row.get("chapters")?,
      order: row.get("ord")?,
    })
  }
}
