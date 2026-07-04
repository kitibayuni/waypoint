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
	import NodeDetailsPanel from '$lib/components/NodeDetailsPanel.svelte';
	import GraphContextMenu from '$lib/components/GraphContextMenu.svelte';
	import RelationshipPopup from '$lib/components/RelationshipPopup.svelte';

	const engagementId = $page.params.id as string;

	let hosts = $state<Host[]>([]);
	let elements = $state<ElementDefinition[]>([]);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<{ id: string; type: string; data: Record<string, unknown> } | null>(null);
	let contextMenu = $state<{
		x: number;
		y: number;
		target: 'background' | 'host' | 'credential';
		nodeId?: string;
	} | null>(null);
	let relationshipDraft = $state<{
		fromHostId: string;
		toHostId: string;
		x: number;
		y: number;
	} | null>(null);

	let showNewHostForm = $state(false);
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
			await createHost(engagementId, {
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
			newLabel = '';
			newHostname = '';
			newOs = '';
			newAddresses = '';
			newTags = '';
			showNewHostForm = false;
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

	<button type="button" class="toggle-form" onclick={() => (showNewHostForm = !showNewHostForm)}>
		{showNewHostForm ? 'Cancel' : '+ New host'}
	</button>

	{#if showNewHostForm}
		<form onsubmit={handleCreate} class="new-host-form">
			<div class="grid">
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
			</div>
			<button type="submit">Add host</button>
		</form>
	{/if}

	<div class="layout">
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
		{#if loading}
			<p>Loading…</p>
		{:else}
			<AttackGraph
				{elements}
				onHostDblClick={handleHostDblClick}
				onNodeSelect={(s) => (selected = s)}
				onContextMenu={(info) => (contextMenu = info)}
				onEdgeCreate={(info) => (relationshipDraft = info)}
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

	{#if contextMenu}
		<GraphContextMenu
			info={contextMenu}
			{engagementId}
			onClose={() => (contextMenu = null)}
			onChanged={load}
		/>
	{/if}

	{#if relationshipDraft}
		<RelationshipPopup
			info={relationshipDraft}
			{engagementId}
			onClose={() => (relationshipDraft = null)}
			onChanged={load}
		/>
	{/if}
</main>

<style>
	.error {
		color: var(--error);
	}
	.muted {
		color: var(--text-muted);
	}
	.toggle-form {
		margin-bottom: 0.75rem;
	}
	.new-host-form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: 0.5rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
	}
	.layout {
		display: grid;
		grid-template-columns: 20rem 1fr;
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
