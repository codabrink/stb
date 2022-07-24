use crate::{init::Verse, search::search};
use anyhow::Result;
use rocket::form::{Context, Contextual, Form, FromForm, FromFormField};
use rocket::fs::{relative, FileServer, TempFile};
use rocket::http::{ContentType, RawStr, Status};
use rocket::time::Date;
use rocket::{Build, Rocket};
use sass_rocket_fairing::SassFairing;

use rocket_dyn_templates::{context, Template};

#[get("/?<q>&<limit>")]
async fn index(q: Option<String>, limit: Option<usize>) -> Template {
  let mut verses: Option<Vec<Verse>> = None;
  if let Some(q) = &q {
    let limit = limit.unwrap_or(20);
    verses = Some(search(q, limit).await.expect("shoot"));
  }

  Template::render("index", context! { verses, title: "Search", query: q })
}

#[get("/book/<slug>/<chapter>")]
async fn chapter(slug: &str, chapter: u64) -> Template {
  verse(slug, chapter, None).await
}

#[get("/book/<slug>/<chapter>/<verse>")]
async fn verse(slug: &str, chapter: u64, verse: Option<u64>) -> Template {
  let verses = Verse::query(&slug, chapter, None).unwrap();
  Template::render("chapter", context! { verses, title: &slug, verse })
}

#[rocket::main]
pub async fn rocket() -> Result<()> {
  let _ = rocket::build()
    .mount("/", routes![index, chapter])
    .attach(Template::fairing())
    .attach(SassFairing::default())
    .mount("/", FileServer::from(relative!("static")))
    .launch()
    .await?;

  Ok(())
}
