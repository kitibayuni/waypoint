<script lang="ts">
	import { listHosts } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { listEngagementChecklists } from '$lib/api/checklists';
	import type { Checklist } from '$lib/api/checklists';
	import ChecklistPanel from '$lib/components/ChecklistPanel.svelte';

	let { engagementId }: { engagementId: string } = $props();

	const PANEL_WIDTH = 384; // 24rem, matches NodeDetailsPanel's default

	let open = $state(false);
	let loading = $state(false);
	let error = $state('');
	let hosts = $state<Host[]>([]);
	let checklists = $state<Checklist[]>([]);
	let view = $state<'hosts' | 'all-todo' | 'host'>('hosts');
	let selectedHostId = $state<string | null>(null);

	const selectedHost = $derived(hosts.find((h) => h.id === selectedHostId) ?? null);

	function checklistsFor(hostId: string): Checklist[] {
		return checklists.filter((c) => c.host_id === hostId);
	}

	function completion(hostId: string): { done: number; total: number } {
		const items = checklistsFor(hostId).flatMap((c) => c.items);
		return { done: items.filter((i) => i.state === 'done' || i.state === 'na').length, total: items.length };
	}

	const todoHosts = $derived(
		hosts
			.map((h) => ({
				host: h,
				checklists: checklistsFor(h.id)
					.map((c) => ({ ...c, items: c.items.filter((i) => i.state === 'todo' || i.state === 'doing') }))
					.filter((c) => c.items.length > 0)
			}))
			.filter((h) => h.checklists.length > 0)
	);

	async function load() {
		loading = true;
		error = '';
		try {
			const [hostList, checklistList] = await Promise.all([
				listHosts(engagementId),
				listEngagementChecklists(engagementId)
			]);
			hosts = hostList;
			checklists = checklistList;
		} catch {
			error = 'Failed to load hosts/checklists.';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (open) load();
	});

	function openHost(hostId: string) {
		selectedHostId = hostId;
		view = 'host';
	}

	function handleHostChecklistChange(updated: Checklist) {
		checklists = checklists.map((c) => (c.id === updated.id ? updated : c));
	}

	// The updated checklist here only carries the todo/doing subset shown in
	// the ALL HOSTS view -- merge its items back into the full checklist by id
	// rather than replacing it wholesale, or the filtered-out done/na items
	// would be dropped from state.
	function handleTodoChecklistChange(updated: Checklist) {
		checklists = checklists.map((c) => {
			if (c.id !== updated.id) return c;
			const byId = new Map(updated.items.map((i) => [i.id, i]));
			return { ...c, items: c.items.map((i) => byId.get(i.id) ?? i) };
		});
	}
</script>

<button
	type="button"
	class="toggle-tab"
	style={`left: ${open ? PANEL_WIDTH : 0}px`}
	onclick={() => (open = !open)}
	aria-label={open ? 'Close checklist panel' : 'Open checklist panel'}
>
	{open ? '‹' : '›'}
</button>

<aside class="panel" class:open style={`width: ${PANEL_WIDTH}px`}>
	<div class="panel-header">
		<h2>Checklists</h2>
		<button type="button" class="all-hosts" onclick={() => (view = 'all-todo')}>ALL HOSTS</button>
	</div>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	{#if loading}
		<p class="muted">Loading…</p>
	{:else if view === 'hosts'}
		{#if hosts.length === 0}
			<p class="muted">No hosts yet.</p>
		{:else}
			<div class="host-list">
				{#each hosts as host (host.id)}
					{@const { done, total } = completion(host.id)}
					<button type="button" class="host-card" onclick={() => openHost(host.id)}>
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
						{#if total > 0}
							<div class="progress">{done}/{total} checklist items done</div>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	{:else if view === 'host'}
		<button type="button" class="back" onclick={() => (view = 'hosts')}>&larr; All hosts</button>
		{#if selectedHost}
			<h3 class="host-heading">{selectedHost.label}</h3>
			{#if checklistsFor(selectedHost.id).length === 0}
				<p class="muted">No checklists yet for this host.</p>
			{:else}
				{#each checklistsFor(selectedHost.id) as checklist (checklist.id)}
					<ChecklistPanel {checklist} onchange={handleHostChecklistChange} />
				{/each}
			{/if}
		{/if}
	{:else}
		<button type="button" class="back" onclick={() => (view = 'hosts')}>&larr; All hosts</button>
		<h3 class="host-heading">All to-do items</h3>
		{#if todoHosts.length === 0}
			<p class="muted">Nothing outstanding — every checklist item is done or n/a.</p>
		{:else}
			{#each todoHosts as entry (entry.host.id)}
				<h4 class="todo-host-heading">{entry.host.label}</h4>
				{#each entry.checklists as checklist (checklist.id)}
					<ChecklistPanel {checklist} onchange={handleTodoChecklistChange} />
				{/each}
			{/each}
		{/if}
	{/if}
</aside>

<style>
	.toggle-tab {
		position: fixed;
		top: 50%;
		transform: translateY(-50%);
		z-index: 51;
		width: 1.5rem;
		height: 3rem;
		border: 1px solid var(--border-strong);
		border-left: none;
		border-radius: 0 6px 6px 0;
		background: var(--surface);
		color: var(--text-muted);
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
		transition: left 0.15s ease-out;
	}
	.toggle-tab:hover {
		color: var(--accent);
	}
	.panel {
		position: fixed;
		top: 0;
		left: 0;
		height: 100dvh;
		max-width: 90vw;
		overflow-y: auto;
		background: var(--surface);
		border-right: 1px solid var(--border);
		box-shadow: 2px 0 12px rgba(0, 0, 0, 0.3);
		padding: 1rem;
		box-sizing: border-box;
		z-index: 50;
		transform: translateX(-100%);
		transition: transform 0.15s ease-out;
	}
	.panel.open {
		transform: translateX(0);
	}
	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
		gap: 0.5rem;
	}
	.panel-header h2 {
		margin: 0;
		font-size: 1.1rem;
	}
	.all-hosts {
		font-size: 0.75rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		background: var(--surface-2);
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		padding: 0.3rem 0.7rem;
		cursor: pointer;
		white-space: nowrap;
	}
	.all-hosts:hover {
		border-color: var(--accent);
	}
	.error {
		color: var(--error);
	}
	.muted {
		color: var(--text-muted);
	}
	.back {
		background: none;
		border: none;
		color: var(--text-muted);
		cursor: pointer;
		font: inherit;
		padding: 0;
		margin-bottom: 0.75rem;
	}
	.back:hover {
		color: var(--accent);
	}
	.host-heading {
		margin: 0 0 0.75rem;
	}
	.todo-host-heading {
		margin: 1rem 0 0.4rem;
		font-size: 0.9rem;
		color: var(--text-muted);
		border-top: 1px solid var(--border);
		padding-top: 0.75rem;
	}
	.todo-host-heading:first-of-type {
		border-top: none;
		padding-top: 0;
		margin-top: 0;
	}
	.host-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.host-card {
		display: block;
		width: 100%;
		text-align: left;
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem 1rem;
		background: none;
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
	.progress {
		margin-top: 0.4rem;
		font-size: 0.8rem;
		color: var(--text-muted);
	}
</style>
