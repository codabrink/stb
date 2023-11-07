use crate::search::search;
use anyhow::Result;
use rocket::http::{CookieJar, RawStr};
use rocket::{fs::FileServer, serde::json::Json};

#[get("/q?<q>&<limit>")]
async fn index(q: &str, limit: Option<usize>, jar: &CookieJar<'_>) -> String {
  dbg!(q);

  let include_apocrypha = match jar.get("include_apocrypha") {
    Some(cookie) if cookie.value() == "true" => true,
    _ => false,
  };

  let a = search(q, 50, include_apocrypha)
    .await
    .expect("failure to search");

  serde_json::to_string_pretty(&a).expect("serilize")
}

pub async fn rocket() -> Result<()> {
  let _ = rocket::build()
    .mount("/", routes![index])
    .mount("/", FileServer::from("static"))
    .launch()
    .await?;

  Ok(())
}
