<script lang="ts">
	import { page } from '$app/stores';
	import { search } from '$lib/api/search';
	import type { SearchResult } from '$lib/api/search';

	const engagementId = $page.params.id as string;

	const allTypes = ['notes', 'findings', 'hosts', 'credentials', 'attachments'];

	let query = $state('');
	let selectedTypes = $state<string[]>([]);
	let results = $state<SearchResult[]>([]);
	let loading = $state(false);
	let error = $state('');
	let searched = $state(false);

	function toggleType(t: string) {
		if (selectedTypes.includes(t)) {
			selectedTypes = selectedTypes.filter((x) => x !== t);
		} else {
			selectedTypes = [...selectedTypes, t];
		}
	}

	async function handleSearch(e: SubmitEvent) {
		e.preventDefault();
		if (!query.trim()) return;
		loading = true;
		error = '';
		searched = true;
		try {
			results = await search(engagementId, query, selectedTypes);
		} catch {
			error = 'Search failed.';
		} finally {
			loading = false;
		}
	}

	function resultHref(result: SearchResult): string | null {
		switch (result.result_type) {
			case 'host':
				return `/engagements/${engagementId}/hosts/${result.id}`;
			case 'finding':
				return `/engagements/${engagementId}/findings/${result.id}`;
			case 'credential':
				return `/engagements/${engagementId}/credentials`;
			default:
				return null;
		}
	}
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Search</h1>

	<form onsubmit={handleSearch} class="search-form">
		<input bind:value={query} placeholder="Search notes, findings, hosts…" required />
		<button type="submit">Search</button>
	</form>

	<div class="type-filters">
		{#each allTypes as t (t)}
			<label class="checkbox">
				<input
					type="checkbox"
					checked={selectedTypes.includes(t)}
					onchange={() => toggleType(t)}
				/>
				{t}
			</label>
		{/each}
		<span class="muted">(none selected = search all)</span>
	</div>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	{#if loading}
		<p>Searching…</p>
	{:else if searched && results.length === 0}
		<p class="muted">No results.</p>
	{:else if results.length > 0}
		<ul class="results">
			{#each results as result (result.result_type + result.id)}
				{@const href = resultHref(result)}
				<li>
					<span class="type-badge">{result.result_type}</span>
					{#if href}
						<a {href}>{result.title}</a>
					{:else}
						<strong>{result.title}</strong>
					{/if}
					{#if result.snippet}
						<p class="snippet">{result.snippet}</p>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</main>

<style>
	.error {
		color: var(--error);
	}
	.muted {
		color: var(--text-muted);
	}
	.search-form {
		display: flex;
		gap: 0.5rem;
		max-width: 32rem;
		margin-bottom: 1rem;
	}
	.search-form input {
		flex: 1;
	}
	.type-filters {
		display: flex;
		gap: 0.75rem;
		flex-wrap: wrap;
		align-items: center;
		margin-bottom: 1.5rem;
		font-size: 0.85rem;
	}
	.checkbox {
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}
	.results {
		list-style: none;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.results li {
		border-bottom: 1px solid var(--border);
		padding-bottom: 0.6rem;
	}
	.type-badge {
		display: inline-block;
		font-size: 0.7rem;
		text-transform: uppercase;
		background: var(--surface-2);
		border-radius: 999px;
		padding: 0.1rem 0.5rem;
		margin-right: 0.5rem;
	}
	.snippet {
		color: var(--text-muted);
		font-size: 0.9rem;
		margin: 0.25rem 0 0;
	}
</style>
