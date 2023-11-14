import type { PageLoad } from "./$types"
import type { Verse } from "../../store"
import { verses } from "./store"

export const load: PageLoad = ({ params }) => {
  fetch(`http://localhost:8080/chapter/${params.book_slug}/${params.chapter}`)
  .then(response => response.json())
  .then(verses.set)

  return {}
}
