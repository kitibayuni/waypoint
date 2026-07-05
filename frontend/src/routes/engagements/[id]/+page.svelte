<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import type { ElementDefinition } from 'cytoscape';
	import { getGraph } from '$lib/api/graph';
	import AttackGraph from '$lib/components/AttackGraph.svelte';
	import NodeDetailsPanel from '$lib/components/NodeDetailsPanel.svelte';
	import GraphContextMenu from '$lib/components/GraphContextMenu.svelte';
	import RelationshipPopup from '$lib/components/RelationshipPopup.svelte';
	import ChecklistSidePanel from '$lib/components/ChecklistSidePanel.svelte';

	const engagementId = $page.params.id as string;

	let elements = $state<ElementDefinition[]>([]);
	let loading = $state(true);
	let error = $state('');
	let selected = $state<{ id: string; type: string; data: Record<string, unknown> } | null>(null);
	let contextMenu = $state<{
		x: number;
		y: number;
		target: 'background' | 'host' | 'credential' | 'service';
		nodeId?: string;
		hostId?: string;
	} | null>(null);
	let relationshipDraft = $state<{
		fromHostId: string;
		toHostId: string;
		x: number;
		y: number;
	} | null>(null);

	async function load() {
		loading = true;
		error = '';
		try {
			const graph = await getGraph(engagementId);
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
</script>

<main>
	<h1>Attack Graph</h1>
	<p class="muted">
		Double-click a host node to open its host page. Right-click the graph for more actions, or
		hold right-click and drag from one host to another to create a relationship.
	</p>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<ChecklistSidePanel {engagementId} />

	<div class="layout">
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
	.layout {
		height: 70vh;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
