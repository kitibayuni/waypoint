<script lang="ts">
	import type { Host } from '$lib/api/hosts';

	let { host }: { host: Host } = $props();
</script>

<a class="host-card" href={`/engagements/${host.engagement_id}/hosts/${host.id}`}>
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
</a>

<style>
	.host-card {
		display: block;
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.75rem 1rem;
		text-decoration: none;
		color: inherit;
	}
	.host-card:hover {
		border-color: #999;
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
		background: #eee;
	}
	.line {
		font-size: 0.9rem;
		color: #555;
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
		background: #eef;
		padding: 0.1rem 0.5rem;
		border-radius: 999px;
	}
</style>
