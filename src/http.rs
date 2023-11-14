use crate::model::Book;
use crate::search::search;
use anyhow::Result;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::http::CookieJar;

mod cors;

#[get("/q?<q>&<limit>")]
async fn get_index(q: &str, limit: Option<usize>, jar: &CookieJar<'_>) -> String {
  // index(q, limit, jar).await
  "{}".to_owned()
}

#[post("/q", data = "<q>")]
async fn index(q: Form<Query<'_>>, jar: &CookieJar<'_>) -> String {
  let include_apocrypha = match jar.get("include_apocrypha") {
    Some(cookie) if cookie.value() == "true" => true,
    _ => false,
  };

  let a = search(q.q, 50, include_apocrypha)
    .await
    .expect("failure to search");

  serde_json::to_string_pretty(&a).expect("serilize")
}

#[get("/chapter/<book_slug>/<chapter>")]
async fn chapter(book_slug: &str, chapter: i32) -> String {
  let verses = Book::chapter(book_slug, chapter).await.unwrap();
  serde_json::to_string(&verses).unwrap_or(format!(r#"{{error: "Could not deserialize verses."}}"#))
}

#[derive(FromForm)]
struct Query<'r> {
  q: &'r str,
}

pub async fn rocket() -> Result<()> {
  let _ = rocket::build()
    .attach(cors::CORS)
    .mount("/", routes![index, get_index, chapter])
    .mount("/", FileServer::from("static"))
    .launch()
    .await?;

  Ok(())
}
