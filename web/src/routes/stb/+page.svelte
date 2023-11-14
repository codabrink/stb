<script lang="ts">
	import type { PageData } from './$types';
	import type { ActionResult } from '@sveltejs/kit';
	import { applyAction, deserialize } from '$app/forms';
	import { verses, type Verse } from './store';

	export let data: PageData;

	async function handleSubmit(event: { currentTarget: EventTarget & HTMLFormElement }) {
		const data = new FormData(event.currentTarget);
		const response = await fetch('http://localhost:8080/q', {
			method: 'POST',
			body: data
		});

		const result: unknown = deserialize(await response.text());

		if (typeof localStorage !== 'undefined') localStorage.setItem('q', data.get('q') as string);

		verses.set(result as Verse[]);
	}
</script>

<svelte:head>
	<title>Search the Book</title>
	<meta name="description" content="A unique way to search the Bible." />
</svelte:head>

<main>
	<form method="POST" on:submit|preventDefault={handleSubmit} class="flex justify-center p-2">
		<input type="text" name="q" placeholder="Search" value={data.q} />
	</form>
	<ul class="flex flex-col gap-4 py-6">
		{#each $verses as verse}
			<li class="verse p-2 px-4 rounded-md cursor-pointer hover:bg-white transition-all">
				<a href="/stb/{verse.book_slug}/{verse.chapter}">
					<h4 class="text-lg font-semibold">
						{verse.book}
						{verse.chapter}:{verse.verse} ({verse.distance})
					</h4>
					{verse.content}
				</a>
			</li>
		{/each}
	</ul>
</main>

<style lang="postcss">
	.verse {
		box-shadow: 0 0 10px rgba(4px, 4px, 4px, 0.05);
		background-color: rgba(255, 255, 255, 0.6);
	}
</style>
