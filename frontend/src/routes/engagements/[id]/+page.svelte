<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import type { ElementDefinition } from 'cytoscape';
	import { getGraph } from '$lib/api/graph';
	import { listHosts } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { createTrustRelationship } from '$lib/api/trust_relationships';
	import AttackGraph from '$lib/components/AttackGraph.svelte';
	import NodeDetailsPanel from '$lib/components/NodeDetailsPanel.svelte';

	const engagementId = $page.params.id as string;

	let elements = $state<ElementDefinition[]>([]);
	let hosts = $state<Host[]>([]);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<{ id: string; type: string; data: Record<string, unknown> } | null>(null);

	let trustFromHostId = $state('');
	let trustToHostId = $state('');
	let trustKind = $state('domain_trust');
	let trustNote = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			const [graph, hostList] = await Promise.all([getGraph(engagementId), listHosts(engagementId)]);
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
			<AttackGraph
				{elements}
				onHostDblClick={handleHostDblClick}
				onNodeSelect={(s) => (selected = s)}
				positions={{ engagementId, persist: true }}
			/>
		{/if}
	</div>

	{#if selected}
		<NodeDetailsPanel
			selection={selected}
			{engagementId}
			onClose={() => (selected = null)}
			onChanged={load}
		/>
	{/if}
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
		height: 70vh;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
