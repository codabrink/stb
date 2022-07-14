use crate::{init::Verse, search::search};
use anyhow::Result;
use rocket::form::{Context, Contextual, Form, FromForm, FromFormField};
use rocket::fs::{relative, FileServer, TempFile};
use rocket::http::{ContentType, RawStr, Status};
use rocket::time::Date;
use rocket::{Build, Rocket};

use rocket_dyn_templates::{context, Template};

#[get("/?<q>&<limit>")]
async fn index(q: Option<String>, limit: Option<usize>) -> Template {
  let mut verses: Option<Vec<Verse>> = None;
  if let Some(q) = q {
    let limit = limit.unwrap_or(20);
    verses = Some(search(q, limit).await.expect("shoot"));
  }

  Template::render("index", context! { verses })
}

struct IndexContext {
  verses: Option<Vec<Verse>>,
}

#[get("/book/<slug>/<chapter>")]
async fn chapter(slug: &str, chapter: u64) -> Template {
  let verses = Verse::query(&slug, chapter, None).unwrap();
  Template::render("chapter", &Context::default())
}

#[rocket::main]
pub async fn rocket() -> Result<()> {
  let _ = rocket::build()
    .mount("/", routes![index, chapter])
    .attach(Template::fairing())
    .mount("/", FileServer::from(relative!("static")))
    .launch()
    .await?;

  Ok(())
}
