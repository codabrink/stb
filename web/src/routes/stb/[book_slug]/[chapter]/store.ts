import { writable } from "svelte/store";
import type { Verse } from "../../store";

export const verses = writable<Verse[]>([]);
