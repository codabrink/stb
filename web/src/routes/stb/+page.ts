import type { PageLoad } from './$types';
import type { Verse } from './store';
import {verses} from './store';

export const load: PageLoad = ({ }) => {
  let q = "";
  if (typeof localStorage !== "undefined") q = localStorage.getItem('q') || ""

  const data = new FormData();
  data.append("q", q);

  fetch("http://localhost:8080/q", {
    method: 'POST',
    body: data
  })
  .then(response => response.json())
  .then(verses.set)
  .catch(console.log);

  return { q }
}
