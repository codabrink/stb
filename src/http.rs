use crate::{prelude::*, search::search};
use anyhow::Result;
use lazy_static::lazy_static;
use rocket::fs::FileServer;
use rocket::http::CookieJar;
use rocket_dyn_templates::{context, Template};
use sass_rocket_fairing::SassFairing;

lazy_static! {
  pub static ref BOOKS: Vec<Book> = Book::all().unwrap();
}

#[get("/?<q>&<limit>")]
async fn index(q: Option<String>, limit: Option<usize>, jar: &CookieJar<'_>) -> Template {
  let include_apocrypha = match jar.get("include_apocrypha") {
    Some(cookie) if cookie.value() == "true" => true,
    _ => false,
  };

  let mut verses: Option<Vec<Verse>> = None;
  if let Some(q) = &q {
    let limit = limit.unwrap_or(20);
    verses = Some(search(q, limit, include_apocrypha).await.expect("shoot"));
  }

  Template::render(
    "index",
    context! {
      verses,
      title: "Search",
      query: q,
      all_books: &*BOOKS
    },
  )
}

#[get("/book/<slug>/<chapter>")]
async fn chapter(slug: &str, chapter: u64, jar: &CookieJar<'_>) -> Template {
  verse(slug, chapter, None, jar).await
}

#[get("/book/<slug>/<chapter>/<verse>")]
async fn verse(slug: &str, chapter: u64, verse: Option<u64>, jar: &CookieJar<'_>) -> Template {
  let include_apocrypha = match jar.get("include_apocrypha") {
    Some(cookie) if cookie.value() == "true" => true,
    _ => false,
  };

  let verses = Verse::query(slug, chapter, None).unwrap();
  let book = Book::query(slug).unwrap();
  let verse = verse.map(|v| verses[v.saturating_sub(1) as usize].clone());

  let mut similar = None;
  if let Some(verse) = &verse {
    similar = Some(search(&verse.content, 5, include_apocrypha).await.unwrap());
  }

  Template::render(
    "chapter",
    context! {
      verses,
      title: &slug,
      book,

      chapter,
      verse,
      similar,
      all_books: &*BOOKS
    },
  )
}

#[rocket::main]
pub async fn rocket() -> Result<()> {
  let _ = rocket::build()
    .mount("/", routes![index, chapter, verse])
    .attach(Template::fairing())
    .attach(SassFairing::default())
    .mount("/", FileServer::from("static"))
    .launch()
    .await?;

  Ok(())
}
