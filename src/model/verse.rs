use crate::init::pg;
use anyhow::Result;
use postgres::Row;
use serde::Serialize;
use std::{fmt::Display, ops::Range};

#[derive(Serialize, Clone)]
pub struct Verse {
  pub id: i32,
  pub verse: i32,
  pub chapter: i32,
  pub book: String,
  pub book_slug: String,
  pub book_order: i32,
  pub content: String,
  pub distance: f64,
}

impl Verse {
  pub fn query(slug: &str, chapter: u32, verses: Option<Range<u32>>) -> Result<Vec<Self>> {
    let mut client = pg()?;
    let mut query = "SELECT * FROM verses WHERE book_slug = (?1) AND chapter = (?2) ".to_string();

    let rows = if let Some(verses) = verses {
      query.push_str("AND verse >= (?3) AND verse <= (?4) ORDER BY verse ASC");
      client.query(&query, &[&slug, &chapter, &verses.start, &verses.end])?
    } else {
      query.push_str("ORDER BY verse ASC");
      client.query(&query, &[&slug, &chapter])?
    };

    Ok(rows.into_iter().map(Verse::parse_row).collect())
  }

  pub fn parse_row(row: Row) -> Self {
    Verse {
      id: row.get("id"),
      content: row.get("content"),
      book: row.get("book"),
      verse: row.get("verse"),
      book_slug: row.get("book_slug"),
      chapter: row.get("chapter"),
      book_order: row.get("book_order"),
      distance: 0.,
    }
  }
}

impl Display for Verse {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} {}:{}", self.book, self.chapter, self.verse)
  }
}
