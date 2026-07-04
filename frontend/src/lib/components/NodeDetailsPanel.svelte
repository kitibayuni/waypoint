<script lang="ts">
	import { getHost, createHost, updateHost } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { getCredential, revealCredential } from '$lib/api/credentials';
	import type { Credential } from '$lib/api/credentials';
	import { createTrustRelationship } from '$lib/api/trust_relationships';

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

	let panelEl = $state<HTMLElement>();
	let host = $state<Host | null>(null);
	let credential = $state<Credential | null>(null);
	let loading = $state(false);
	let error = $state('');
	let revealedSecret = $state('');

	let newLabel = $state('');
	let newHostname = $state('');
	let newAddresses = $state('');
	let newKind = $state('session');

	async function load() {
		host = null;
		credential = null;
		revealedSecret = '';
		error = '';
		if (!selection) return;
		loading = true;
		try {
			if (selection.type === 'host') {
				host = await getHost(selection.id);
			} else if (selection.type === 'credential') {
				credential = await getCredential(selection.id);
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
		if (!selection) return;
		const target = e.target as Node;
		if (panelEl && !panelEl.contains(target) && !(target as Element).closest?.('.graph-container')) {
			onClose();
		}
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
</script>

<svelte:document onclick={handleDocumentClick} />

{#if selection}
	<aside class="panel" class:open={!!selection} bind:this={panelEl}>
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
		{/if}
	</aside>
{/if}

<style>
	.panel {
		position: fixed;
		top: 0;
		right: 0;
		height: 100dvh;
		width: 24rem;
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
</style>
