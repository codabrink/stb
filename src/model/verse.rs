use anyhow::Result;
use serde::Serialize;
use std::{fmt::Display, ops::Range};
use tokio_postgres::{NoTls, Row};

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
  pub async fn query(slug: &str, chapter: u32, verses: Option<Range<u32>>) -> Result<Vec<Self>> {
    let (client, connection) =
      tokio_postgres::connect("postgresql://postgres:postgres@localhost/stb", NoTls).await?;

    // TODO: super bad, fix later
    tokio::spawn(async move {
      if let Err(e) = connection.await {
        eprintln!("connection error: {}", e);
      }
    });

    let mut query = "SELECT * FROM verses WHERE book_slug = (?1) AND chapter = (?2) ".to_string();

    let rows = if let Some(verses) = verses {
      query.push_str("AND verse >= (?3) AND verse <= (?4) ORDER BY verse ASC");
      client
        .query(&query, &[&slug, &chapter, &verses.start, &verses.end])
        .await?
    } else {
      query.push_str("ORDER BY verse ASC");
      client.query(&query, &[&slug, &chapter]).await?
    };

    Ok(rows.into_iter().map(Verse::from).collect())
  }
}

impl From<&Row> for Verse {
  #[inline]
  fn from(row: &Row) -> Self {
    Self {
      id: row.get("id"),
      content: row.get("content"),
      book: row.get("book"),
      verse: row.get("verse"),
      book_slug: row.get("book_slug"),
      chapter: row.get("chapter"),
      book_order: row.get("book_order"),
      distance: row.try_get("distance").unwrap_or(0.),
    }
  }
}

impl From<Row> for Verse {
  fn from(row: Row) -> Self {
    Self::from(&row)
  }
}

impl Display for Verse {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} {}:{}", self.book, self.chapter, self.verse)
  }
}
