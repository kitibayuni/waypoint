<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import type { ElementDefinition } from 'cytoscape';
	import { listHosts, createHost } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { getGraph } from '$lib/api/graph';
	import HostCard from '$lib/components/HostCard.svelte';
	import AttackGraph from '$lib/components/AttackGraph.svelte';

	const engagementId = $page.params.id as string;

	let hosts = $state<Host[]>([]);
	let elements = $state<ElementDefinition[]>([]);
	let loading = $state(true);
	let error = $state('');

	let newLabel = $state('');
	let newHostname = $state('');
	let newOs = $state('');
	let newAddresses = $state('');
	let newTags = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			const [hostList, graph] = await Promise.all([listHosts(engagementId), getGraph(engagementId)]);
			hosts = hostList;
			elements = [
				...graph.nodes.filter((n) => n.data.type === 'host'),
				...graph.edges.filter((e) => e.data.type === 'trust')
			] as ElementDefinition[];
		} catch {
			error = 'Failed to load hosts.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	function handleHostDblClick(hostId: string) {
		goto(`/engagements/${engagementId}/hosts/${hostId}`);
	}

	async function handleCreate(e: SubmitEvent) {
		e.preventDefault();
		if (!newLabel.trim()) return;
		try {
			const host = await createHost(engagementId, {
				label: newLabel,
				hostname: newHostname || null,
				os: newOs || null,
				addresses: newAddresses
					.split(',')
					.map((s) => s.trim())
					.filter(Boolean),
				tags: newTags
					.split(',')
					.map((s) => s.trim())
					.filter(Boolean)
			});
			hosts = [...hosts, host];
			newLabel = '';
			newHostname = '';
			newOs = '';
			newAddresses = '';
			newTags = '';
			error = '';
			await load();
		} catch {
			error = 'Failed to create host (check the IP address format).';
		}
	}
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Hosts</h1>
	<p class="muted">Double-click a host node to open its host page.</p>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<form onsubmit={handleCreate}>
		<h2>New host</h2>
		<label>
			Label
			<input bind:value={newLabel} required placeholder="e.g. DC01" />
		</label>
		<label>
			Hostname
			<input bind:value={newHostname} placeholder="e.g. dc01.corp.local" />
		</label>
		<label>
			OS
			<input bind:value={newOs} placeholder="e.g. Windows Server 2019" />
		</label>
		<label>
			IP addresses (comma-separated)
			<input bind:value={newAddresses} placeholder="10.10.10.5, 10.10.10.6" />
		</label>
		<label>
			Tags (comma-separated)
			<input bind:value={newTags} placeholder="domain-controller, critical" />
		</label>
		<button type="submit">Add host</button>
	</form>

	<div class="layout">
		{#if loading}
			<p>Loading…</p>
		{:else}
			<AttackGraph {elements} onHostDblClick={handleHostDblClick} />
		{/if}
		<aside class="host-sidebar">
			<h2>All hosts</h2>
			{#if hosts.length === 0}
				<p class="muted">No hosts yet.</p>
			{:else}
				<div class="host-list">
					{#each hosts as host (host.id)}
						<HostCard {host} />
					{/each}
				</div>
			{/if}
		</aside>
	</div>
</main>

<style>
	.error {
		color: var(--error);
	}
	.muted {
		color: var(--text-muted);
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-width: 24rem;
		margin-bottom: 1.5rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
	}
	.layout {
		display: grid;
		grid-template-columns: 1fr 20rem;
		grid-template-rows: minmax(0, 1fr);
		gap: 1rem;
		height: 70vh;
	}
	.host-sidebar {
		height: 100%;
		overflow-y: auto;
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem;
		box-sizing: border-box;
	}
	.host-sidebar h2 {
		margin-top: 0;
	}
	.host-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
</style>
