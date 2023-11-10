use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::{fmt::Display, ops::Range};
use tokio_postgres::Row;

static SPLIT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r",|\.").unwrap());

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
    let client = crate::db::POOL.get().await?;

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

  pub fn shatter(&self) -> Result<Vec<String>> {
    let mut fragments = vec![self.content.clone()];

    let splits: Vec<String> = SPLIT_REGEX
      .split(&self.content)
      .map(|s| {
        s.chars()
          .filter(|c| *c == ' ' || *c == '\'' || c.is_ascii_alphanumeric())
          .collect()
      })
      .collect();

    if splits.len() > 1 {
      for split in &splits {
        if split.len() < 8 {
          continue;
        }
        println!("SPLIT: {}", &split.trim());
        fragments.push(split.trim().to_owned());
      }
    }

    for i in 1..(splits.len() - 1) {
      let subverse = splits[i..]
        .iter()
        .map(|s| s.trim())
        .collect::<Vec<&str>>()
        .join(" ");

      println!("SUBVERSE: {}", &subverse);
      fragments.push(subverse);
    }

    println!(
      "Shattered verse {} {}:{} ({} splits)",
      self.book,
      self.chapter,
      self.verse,
      &splits.len()
    );

    Ok(fragments)
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
