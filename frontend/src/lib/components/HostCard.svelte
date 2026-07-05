<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Host } from '$lib/api/hosts';

	// `onclick` switches this from a navigation link (the Hosts page's default
	// use) to a plain button that lets the caller handle the click itself
	// (e.g. ChecklistSidePanel switching its own internal view instead of
	// leaving the page). `extra` lets a caller append content below the tags
	// (e.g. a checklist-completion badge) without forking this component.
	let { host, onclick, extra }: { host: Host; onclick?: () => void; extra?: Snippet } = $props();
</script>

{#snippet content()}
	<div class="host-card-header">
		<strong>{host.label}</strong>
		<span class="status">{host.status}</span>
	</div>
	{#if host.hostname}<div class="line">{host.hostname}</div>{/if}
	{#if host.os}<div class="line">{host.os}</div>{/if}
	{#if host.addresses.length}
		<div class="line addresses">{host.addresses.map((a) => a.ip).join(', ')}</div>
	{/if}
	{#if host.tags.length}
		<div class="tags">
			{#each host.tags as tag (tag.id)}
				<span class="tag">{tag.name}</span>
			{/each}
		</div>
	{/if}
	{#if extra}{@render extra()}{/if}
{/snippet}

{#if onclick}
	<button type="button" class="host-card" {onclick}>{@render content()}</button>
{:else}
	<a class="host-card" href={`/engagements/${host.engagement_id}/hosts/${host.id}`}>
		{@render content()}
	</a>
{/if}

<style>
	.host-card {
		display: block;
		width: 100%;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem 1rem;
		background: none;
		text-decoration: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}
	.host-card:hover {
		border-color: var(--text-muted);
	}
	.host-card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.status {
		font-size: 0.75rem;
		text-transform: uppercase;
		padding: 0.1rem 0.5rem;
		border-radius: 999px;
		background: var(--surface-2);
	}
	.line {
		font-size: 0.9rem;
		color: var(--text-muted);
	}
	.addresses {
		font-family: monospace;
	}
	.tags {
		margin-top: 0.4rem;
		display: flex;
		gap: 0.3rem;
		flex-wrap: wrap;
	}
	.tag {
		font-size: 0.75rem;
		background: var(--surface-3);
		padding: 0.1rem 0.5rem;
		border-radius: 999px;
	}
</style>
