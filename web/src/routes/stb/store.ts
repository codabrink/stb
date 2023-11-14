import { writable } from "svelte/store";

export type Verse = {
  id: number,
  verse: number,
  chapter: number,
  book: string,
  book_slug: string,
  book_order: number,
  content: string,
  distance: number
}

export const verses = writable<Verse[]>([]);
