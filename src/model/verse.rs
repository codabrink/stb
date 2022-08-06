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
    let mut query = String::from(
      "SELECT id, content, book, verse FROM verses WHERE slug = (?1) AND chapter = (?2) ",
    );

    let parser = |row: &Row| {
      Ok(Verse {
        id: row.get(0)?,
        content: row.get(1)?,
        book: row.get(2)?,
        verse: row.get(3)?,
        slug: slug.to_string(),
        chapter: chapter,
      })
    };

    let mut stmt;
    let verses_iter = if let Some(verses) = verses {
      query.push_str("AND verse >= (?3) AND verse <= (?4) ORDER BY verse ASC");
      stmt = conn.prepare(&query)?;
      stmt.query_map(params![slug, chapter, verses.start, verses.end], parser)
    } else {
      query.push_str("ORDER BY verse ASC");
      stmt = conn.prepare(&query)?;
      stmt.query_map(params![slug, chapter], parser)
    }?;

    Ok(verses_iter.flat_map(|v| v).collect())
  }
}
