<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import type { ElementDefinition } from 'cytoscape';
	import { getGraph } from '$lib/api/graph';
	import type { GraphSuggestion } from '$lib/api/graph';
	import { listHosts } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { createTrustRelationship } from '$lib/api/trust_relationships';
	import AttackGraph from '$lib/components/AttackGraph.svelte';

	const engagementId = $page.params.id as string;

	let elements = $state<ElementDefinition[]>([]);
	let suggestions = $state<GraphSuggestion[]>([]);
	let hosts = $state<Host[]>([]);
	let loading = $state(true);
	let error = $state('');

	let trustFromHostId = $state('');
	let trustToHostId = $state('');
	let trustKind = $state('domain_trust');
	let trustNote = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			const [graph, hostList] = await Promise.all([getGraph(engagementId), listHosts(engagementId)]);
			suggestions = graph.suggestions;
			hosts = hostList;
			elements = [...graph.nodes, ...graph.edges] as ElementDefinition[];
		} catch {
			error = 'Failed to load attack graph.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	function handleHostDblClick(hostId: string) {
		goto(`/engagements/${engagementId}/hosts/${hostId}`);
	}

	async function handleAddTrust(e: SubmitEvent) {
		e.preventDefault();
		if (!trustFromHostId || !trustToHostId) return;
		try {
			await createTrustRelationship(engagementId, {
				from_host_id: trustFromHostId,
				to_host_id: trustToHostId,
				kind: trustKind,
				note: trustNote || null
			});
			trustFromHostId = '';
			trustToHostId = '';
			trustNote = '';
			error = '';
			await load();
		} catch {
			error = 'Failed to add trust relationship.';
		}
	}
</script>

<main>
	<h1>Attack Graph</h1>
	<p class="muted">Double-click a host node to open its host page.</p>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<form onsubmit={handleAddTrust} class="trust-form">
		<select bind:value={trustFromHostId}>
			<option value="" disabled selected>From host…</option>
			{#each hosts as host (host.id)}
				<option value={host.id}>{host.label}</option>
			{/each}
		</select>
		<select bind:value={trustKind}>
			<option value="domain_trust">domain trust</option>
			<option value="admin_of">admin of</option>
			<option value="shares_creds">shares creds</option>
			<option value="session">session</option>
		</select>
		<select bind:value={trustToHostId}>
			<option value="" disabled selected>To host…</option>
			{#each hosts as host (host.id)}
				<option value={host.id}>{host.label}</option>
			{/each}
		</select>
		<input bind:value={trustNote} placeholder="note (optional)" />
		<button type="submit">Add relationship</button>
	</form>

	<div class="layout">
		{#if loading}
			<p>Loading…</p>
		{:else}
			<AttackGraph {elements} onHostDblClick={handleHostDblClick} />
		{/if}
		<aside class="suggestions">
			<h2>Suggested next steps</h2>
			{#if loading}
				<p>Loading…</p>
			{:else if suggestions.length === 0}
				<p class="muted">No suggestions yet — confirm an observation to see attack paths.</p>
			{:else}
				<ul>
					{#each suggestions as s}
						<li>
							<strong>{s.host_label}</strong>: {s.observation_title}
							<br />&rarr; <em>{s.technique}</em> &rarr; {s.outcome}
							{#if s.next_step_md}
								<p class="next-step">{s.next_step_md}</p>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</aside>
	</div>
</main>

<style>
	.error {
		color: var(--error);
	}
	.trust-form {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		align-items: center;
		margin-bottom: 1rem;
	}
	.layout {
		display: grid;
		grid-template-columns: 1fr 20rem;
		grid-template-rows: minmax(0, 1fr);
		gap: 1rem;
		height: 70vh;
	}
	.suggestions {
		height: 100%;
		overflow-y: auto;
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem;
		box-sizing: border-box;
	}
	.suggestions ul {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.suggestions li {
		border-bottom: 1px solid var(--border);
		padding-bottom: 0.5rem;
	}
	.next-step {
		font-size: 0.85rem;
		color: var(--text-muted);
		margin: 0.3rem 0 0;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
