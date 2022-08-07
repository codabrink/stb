use crate::prelude::*;

#[derive(Serialize, Clone)]
pub struct Verse {
  pub id: u64,
  pub verse: u64,
  pub chapter: u64,
  pub book: String,
  pub slug: String,
  pub content: String,
}

impl Verse {
  pub fn query(slug: &str, chapter: u64, verses: Option<Range<u64>>) -> Result<Vec<Self>> {
    let conn = Connection::open(SQLITE_DB)?;
    let mut query = String::from("SELECT * FROM verses WHERE slug = (?1) AND chapter = (?2) ");

    let mut stmt;
    let verses_iter = if let Some(verses) = verses {
      query.push_str("AND verse >= (?3) AND verse <= (?4) ORDER BY verse ASC");
      stmt = conn.prepare(&query)?;
      stmt.query_map(
        params![slug, chapter, verses.start, verses.end],
        Self::parse_row,
      )
    } else {
      query.push_str("ORDER BY verse ASC");
      stmt = conn.prepare(&query)?;
      stmt.query_map(params![slug, chapter], Self::parse_row)
    }?;

    Ok(verses_iter.flat_map(|v| v).collect())
  }

  pub fn parse_row(row: &Row) -> Result<Self, rusqlite::Error> {
    Ok(Verse {
      id: row.get("id")?,
      content: row.get("content")?,
      book: row.get("book")?,
      verse: row.get("verse")?,
      slug: row.get("slug")?,
      chapter: row.get("chapter")?,
    })
  }
}
