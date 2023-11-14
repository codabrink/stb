<script lang="ts">
	import { verses } from './store';
</script>

<main>
	{#each $verses as verse}
		{verse.content}
	{/each}
</main>
