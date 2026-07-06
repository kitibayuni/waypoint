<script lang="ts">
	import { getHost, createHost, updateHost } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { getCredential, revealCredential } from '$lib/api/credentials';
	import type { Credential } from '$lib/api/credentials';
	import {
		createTrustRelationship,
		updateTrustRelationship,
		deleteTrustRelationship
	} from '$lib/api/trust_relationships';
	import { getServiceChecklist } from '$lib/api/checklists';
	import type { Checklist } from '$lib/api/checklists';
	import ChecklistPanel from '$lib/components/ChecklistPanel.svelte';
	import {
		listServiceTechnologies,
		createServiceTechnology,
		deleteServiceTechnology,
		KNOWN_TECHNOLOGIES
	} from '$lib/api/service_technologies';
	import type { ServiceTechnology } from '$lib/api/service_technologies';

	let {
		selection,
		engagementId,
		onClose,
		onChanged
	}: {
		selection: { id: string; type: string; data: Record<string, unknown> } | null;
		engagementId: string;
		onClose: () => void;
		onChanged: () => void;
	} = $props();

	const MIN_PANEL_WIDTH = 280;
	const MAX_PANEL_WIDTH = 720;
	const DEFAULT_PANEL_WIDTH = 384; // 24rem

	let panelEl = $state<HTMLElement>();
	let host = $state<Host | null>(null);
	let credential = $state<Credential | null>(null);
	let serviceChecklist = $state<Checklist | null>(null);
	let technologies = $state<ServiceTechnology[]>([]);
	let loading = $state(false);
	let error = $state('');
	let revealedSecret = $state('');
	let panelWidth = $state(
		typeof localStorage !== 'undefined'
			? Number(localStorage.getItem('nodeDetailsPanelWidth')) || DEFAULT_PANEL_WIDTH
			: DEFAULT_PANEL_WIDTH
	);
	let resizing = $state(false);

	let newLabel = $state('');
	let newHostname = $state('');
	let newAddresses = $state('');
	let newKind = $state('session');

	let trustKindDraft = $state('session');
	let trustNoteDraft = $state('');

	let newTechName = $state('');
	let newTechVersion = $state('');
	let newTechNotes = $state('');

	async function load() {
		host = null;
		credential = null;
		serviceChecklist = null;
		technologies = [];
		revealedSecret = '';
		error = '';
		if (!selection) return;
		loading = true;
		try {
			if (selection.type === 'host') {
				host = await getHost(selection.id);
			} else if (selection.type === 'credential') {
				credential = await getCredential(selection.id);
			} else if (selection.type === 'service') {
				// A 404 here just means this service's type has no mapped checklist
				// template (or none instantiated yet) -- an expected, non-error state.
				serviceChecklist = await getServiceChecklist(selection.id).catch(() => null);
				technologies = await listServiceTechnologies(selection.id);
			} else if (selection.type === 'trust') {
				// Already fully present in the edge's own data -- no fetch needed.
				trustKindDraft = (selection.data.kind as string) ?? 'session';
				trustNoteDraft = (selection.data.note as string) ?? '';
			}
		} catch {
			error = 'Failed to load details.';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void selection;
		load();
	});

	function handleDocumentClick(e: MouseEvent) {
		if (!selection || resizing) return;
		const target = e.target as Node;
		if (panelEl && !panelEl.contains(target) && !(target as Element).closest?.('.graph-container')) {
			onClose();
		}
	}

	function startResize(e: MouseEvent) {
		e.preventDefault();
		resizing = true;
		const startX = e.clientX;
		const startWidth = panelWidth;

		function onMove(ev: MouseEvent) {
			// Panel is anchored to the right edge, so dragging left (negative dx)
			// should widen it.
			const dx = startX - ev.clientX;
			panelWidth = Math.min(MAX_PANEL_WIDTH, Math.max(MIN_PANEL_WIDTH, startWidth + dx));
		}
		function onUp() {
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);
			localStorage.setItem('nodeDetailsPanelWidth', String(panelWidth));
			// The mouseup ending this drag also fires a native click afterward;
			// defer clearing `resizing` so handleDocumentClick still ignores that
			// one click instead of misreading it as an outside click that should
			// close the panel (same class of bug fixed earlier for RelationshipPopup).
			setTimeout(() => (resizing = false), 0);
		}
		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	async function toggleFoothold() {
		if (!host) return;
		try {
			host = await updateHost(host.id, { ...host, is_foothold: !host.is_foothold });
			onChanged();
		} catch {
			error = 'Failed to update foothold status.';
		}
	}

	async function togglePivot() {
		if (!host) return;
		try {
			host = await updateHost(host.id, { ...host, is_pivot: !host.is_pivot });
			onChanged();
		} catch {
			error = 'Failed to update pivot status.';
		}
	}

	async function handleReveal() {
		if (!credential) return;
		try {
			const { secret } = await revealCredential(credential.id);
			revealedSecret = secret;
		} catch {
			error = 'Failed to reveal secret.';
		}
	}

	async function handleAddHost(e: SubmitEvent) {
		e.preventDefault();
		if (!newLabel.trim() || !host) return;
		try {
			const created = await createHost(engagementId, {
				label: newLabel,
				addresses: newAddresses
					.split(',')
					.map((s) => s.trim())
					.filter(Boolean),
				hostname: newHostname || null
			});
			await createTrustRelationship(engagementId, {
				from_host_id: host.id,
				to_host_id: created.id,
				kind: newKind,
				note: null
			});
			newLabel = '';
			newHostname = '';
			newAddresses = '';
			error = '';
			onChanged();
		} catch {
			error = 'Failed to add host (check the IP address format).';
		}
	}

	async function handleAddTechnology(e: SubmitEvent) {
		e.preventDefault();
		if (!selection || !newTechName.trim()) return;
		try {
			const tech = await createServiceTechnology(selection.id, {
				name: newTechName.trim(),
				version: newTechVersion || null,
				notes_md: newTechNotes
			});
			technologies = [...technologies, tech];
			newTechName = '';
			newTechVersion = '';
			newTechNotes = '';
			error = '';
			onChanged();
		} catch {
			error = 'Failed to add detected technology.';
		}
	}

	async function handleDeleteTechnology(id: string) {
		try {
			await deleteServiceTechnology(id);
			technologies = technologies.filter((t) => t.id !== id);
		} catch {
			error = 'Failed to remove detected technology.';
		}
	}

	async function handleUpdateTrust(e: SubmitEvent) {
		e.preventDefault();
		if (!selection) return;
		try {
			await updateTrustRelationship(selection.id, {
				from_host_id: (selection.data.source as string).replace(/^host:/, ''),
				to_host_id: (selection.data.target as string).replace(/^host:/, ''),
				kind: trustKindDraft,
				note: trustNoteDraft || null
			});
			error = '';
			onChanged();
		} catch {
			error = 'Failed to update relationship.';
		}
	}

	async function handleDeleteTrust() {
		if (!selection) return;
		if (!confirm('Delete this relationship? This cannot be undone.')) return;
		try {
			await deleteTrustRelationship(selection.id);
			onChanged();
			onClose();
		} catch {
			error = 'Failed to delete relationship.';
		}
	}
</script>

<svelte:document onclick={handleDocumentClick} />

{#if selection}
	<aside
		class="panel"
		class:open={!!selection}
		bind:this={panelEl}
		style={`width: ${panelWidth}px`}
	>
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class="resize-handle"
			onmousedown={startResize}
			role="separator"
			aria-orientation="vertical"
			aria-label="Resize panel"
		></div>
		<div class="panel-header">
			<h2>{selection.data.label}</h2>
			<button type="button" class="close" onclick={onClose} aria-label="Close">&times;</button>
		</div>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		{#if loading}
			<p class="muted">Loading…</p>
		{:else if host}
			<dl>
				<dt>Status</dt>
				<dd>
					{host.status}
					{#if host.is_foothold} · <span class="badge">foothold</span>{/if}
					{#if host.is_pivot} · <span class="badge pivot">pivot</span>{/if}
				</dd>
				{#if host.hostname}
					<dt>Hostname</dt>
					<dd>{host.hostname}</dd>
				{/if}
				{#if host.os}
					<dt>OS</dt>
					<dd>{host.os}</dd>
				{/if}
				{#if host.addresses.length > 0}
					<dt>Addresses</dt>
					<dd>{host.addresses.map((a) => a.ip).join(', ')}</dd>
				{/if}
				{#if host.tags.length > 0}
					<dt>Tags</dt>
					<dd>{host.tags.map((t) => t.name).join(', ')}</dd>
				{/if}
			</dl>
			<p><a href={`/engagements/${engagementId}/hosts/${host.id}`}>Open host page &rarr;</a></p>

			<label class="toggle-row">
				<input type="checkbox" checked={host.is_foothold} onchange={toggleFoothold} />
				Foothold / initial access
			</label>
			<label class="toggle-row">
				<input type="checkbox" checked={host.is_pivot} onchange={togglePivot} />
				Pivot point
			</label>

			<h3>Add host accessible from {host.label}</h3>
			<form onsubmit={handleAddHost}>
				<label>
					Label
					<input bind:value={newLabel} required placeholder="e.g. WEB02" />
				</label>
				<label>
					Hostname
					<input bind:value={newHostname} placeholder="e.g. web02.corp.local" />
				</label>
				<label>
					IP addresses (comma-separated)
					<input bind:value={newAddresses} placeholder="10.10.10.7" />
				</label>
				<label>
					Relationship kind
					<select bind:value={newKind}>
						<option value="session">session</option>
						<option value="domain_trust">domain trust</option>
						<option value="admin_of">admin of</option>
						<option value="shares_creds">shares creds</option>
					</select>
				</label>
				<button type="submit">Add host</button>
			</form>
		{:else if credential}
			<dl>
				{#if credential.domain}
					<dt>Domain</dt>
					<dd>{credential.domain}</dd>
				{/if}
				<dt>Type</dt>
				<dd>{credential.secret_type}</dd>
				<dt>Origin</dt>
				<dd>{credential.origin}</dd>
				<dt>Validated</dt>
				<dd>{credential.validated ? 'yes' : 'no'}</dd>
				{#if credential.notes_md}
					<dt>Notes</dt>
					<dd>{credential.notes_md}</dd>
				{/if}
			</dl>
			<div class="secret-row">
				<button type="button" onclick={handleReveal}>Reveal secret</button>
				{#if revealedSecret}
					<code>{revealedSecret}</code>
				{/if}
			</div>
		{:else if selection.type === 'service'}
			<dl>
				<dt>Port</dt>
				<dd>{selection.data.port}/{selection.data.protocol}</dd>
				{#if selection.data.name}
					<dt>Service</dt>
					<dd>{selection.data.name}</dd>
				{/if}
			</dl>
			{#if selection.data.host_id}
				<p>
					<a href={`/engagements/${engagementId}/hosts/${selection.data.host_id}`}>
						Open host's Services tab &rarr;
					</a>
				</p>
			{/if}

			{#if serviceChecklist}
				<ChecklistPanel
					checklist={serviceChecklist}
					onchange={(updated) => (serviceChecklist = updated)}
				/>
			{:else}
				<p class="muted">No checklist for this service.</p>
			{/if}

			<h3>Detected technology</h3>
			{#if technologies.length === 0}
				<p class="muted">Nothing detected on this service yet.</p>
			{:else}
				<ul class="tech-list">
					{#each technologies as t (t.id)}
						<li>
							{t.name}{t.version ? ` ${t.version}` : ''}
							{#if t.notes_md}<span class="muted"> — {t.notes_md}</span>{/if}
							<button type="button" onclick={() => handleDeleteTechnology(t.id)}>&times;</button>
						</li>
					{/each}
				</ul>
			{/if}
			<form onsubmit={handleAddTechnology}>
				<label>
					Name
					<input bind:value={newTechName} required list="known-technologies" placeholder="e.g. wordpress" />
					<datalist id="known-technologies">
						{#each KNOWN_TECHNOLOGIES as name (name)}
							<option value={name}></option>
						{/each}
					</datalist>
				</label>
				<label>
					Version
					<input bind:value={newTechVersion} placeholder="optional" />
				</label>
				<label>
					Notes
					<input bind:value={newTechNotes} placeholder="optional" />
				</label>
				<button type="submit">Add</button>
			</form>
		{:else if selection.type === 'trust'}
			<form onsubmit={handleUpdateTrust}>
				<label>
					Kind
					<select bind:value={trustKindDraft}>
						<option value="domain_trust">domain trust</option>
						<option value="admin_of">admin of</option>
						<option value="shares_creds">shares creds</option>
						<option value="session">session</option>
					</select>
				</label>
				<label>
					Note
					<input bind:value={trustNoteDraft} placeholder="note (optional)" />
				</label>
				<div class="secret-row">
					<button type="submit">Save</button>
					<button type="button" onclick={handleDeleteTrust}>Delete</button>
				</div>
			</form>
		{/if}
	</aside>
{/if}

<style>
	.panel {
		position: fixed;
		top: 0;
		right: 0;
		height: 100dvh;
		max-width: 90vw;
		overflow-y: auto;
		background: var(--surface);
		border-left: 1px solid var(--border);
		box-shadow: -2px 0 12px rgba(0, 0, 0, 0.3);
		padding: 1rem;
		box-sizing: border-box;
		z-index: 50;
		transform: translateX(0);
		transition: transform 0.15s ease-out;
	}
	.resize-handle {
		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		width: 6px;
		cursor: ew-resize;
		z-index: 51;
	}
	.resize-handle:hover {
		background: var(--accent);
		opacity: 0.5;
	}
	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.5rem;
	}
	.panel-header h2 {
		margin: 0;
		font-size: 1.1rem;
	}
	.close {
		background: none;
		border: none;
		font-size: 1.4rem;
		line-height: 1;
		cursor: pointer;
		color: var(--text-muted);
		padding: 0.1rem 0.4rem;
	}
	.error {
		color: var(--error);
	}
	.muted {
		color: var(--text-muted);
	}
	.badge {
		color: var(--error);
		font-weight: 600;
	}
	.badge.pivot {
		color: var(--warning);
	}
	.toggle-row {
		flex-direction: row;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.85rem;
		margin-bottom: 0.3rem;
	}
	dl {
		margin: 0 0 0.75rem;
		font-size: 0.85rem;
	}
	dt {
		color: var(--text-muted);
		margin-top: 0.4rem;
	}
	dd {
		margin: 0;
	}
	h3 {
		font-size: 0.95rem;
		margin-bottom: 0.4rem;
		border-top: 1px solid var(--border);
		padding-top: 0.75rem;
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		font-size: 0.85rem;
	}
	.secret-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	.tech-list {
		list-style: none;
		padding: 0;
		margin: 0 0 0.75rem;
		font-size: 0.85rem;
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.tech-list button {
		background: none;
		border: none;
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
	}
</style>
