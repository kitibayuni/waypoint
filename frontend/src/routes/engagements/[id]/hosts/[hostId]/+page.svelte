<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import {
		getHost,
		updateHost,
		addAddress,
		removeAddress,
		addTag,
		removeTag,
		listHosts
	} from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { listServices, createService, deleteService, SERVICE_NAMES } from '$lib/api/services';
	import type { Service } from '$lib/api/services';
	import {
		listTrustRelationships,
		createTrustRelationship,
		deleteTrustRelationship
	} from '$lib/api/trust_relationships';
	import type { TrustRelationship } from '$lib/api/trust_relationships';
	import { listPivotTunnels, createPivotTunnel, deletePivotTunnel, PIVOT_METHODS } from '$lib/api/pivot_tunnels';
	import type { PivotTunnel } from '$lib/api/pivot_tunnels';
	import { listHostChecklists } from '$lib/api/checklists';
	import type { Checklist } from '$lib/api/checklists';
	import ChecklistPanel from '$lib/components/ChecklistPanel.svelte';
	import { listNotes } from '$lib/api/notes';
	import type { Note } from '$lib/api/notes';
	import NoteEditor from '$lib/components/NoteEditor.svelte';
	import { listAttachments } from '$lib/api/attachments';
	import type { Attachment } from '$lib/api/attachments';
	import AttachmentGallery from '$lib/components/AttachmentGallery.svelte';

	const engagementId = $page.params.id as string;
	const hostId = $page.params.hostId as string;

	let host = $state<Host | null>(null);
	let services = $state<Service[]>([]);
	let loading = $state(true);
	let error = $state('');
	let activeTab = $state<'general' | 'services' | 'checklists' | 'notes' | 'attachments'>(
		'general'
	);

	let labelDraft = $state('');
	let hostnameDraft = $state('');
	let osDraft = $state('');
	let osFamilyDraft = $state('');
	let criticalityDraft = $state('');
	let statusDraft = $state('discovered');
	let notesDraft = $state('');
	let loginNotesDraft = $state('');
	let isFootholdDraft = $state(false);
	let isPivotDraft = $state(false);

	let allHosts = $state<Host[]>([]);
	let allTrust = $state<TrustRelationship[]>([]);
	let incomingAccess = $derived(allTrust.filter((t) => t.to_host_id === hostId));
	let accessDirection = $state<'from' | 'to'>('from');
	let accessOtherHostId = $state('');
	let accessKind = $state('session');
	let accessNote = $state('');

	let allPivots = $state<PivotTunnel[]>([]);
	let pivotsFromHost = $derived(allPivots.filter((p) => p.from_host_id === hostId));
	let newPivotToHostId = $state('');
	let newPivotMethod = $state('ssh_dynamic');
	let newPivotLocalPort = $state<number | ''>('');
	let newPivotRemoteTarget = $state('');
	let newPivotNotes = $state('');

	let newIp = $state('');
	let newTagName = $state('');

	let newPort = $state<number | ''>('');
	let newProtocol = $state('tcp');
	let newServiceName = $state('');
	let newDisplayName = $state('');
	let newVersion = $state('');

	let checklists = $state<Checklist[]>([]);
	let notes = $state<Note[]>([]);
	let attachments = $state<Attachment[]>([]);

	async function load() {
		loading = true;
		error = '';
		try {
			const [h, svc, cls, hostNotes, hostAttachments, hosts, trust, pivots] = await Promise.all([
				getHost(hostId),
				listServices(hostId),
				listHostChecklists(hostId),
				listNotes(engagementId, 'host', hostId),
				listAttachments(engagementId, 'host', hostId),
				listHosts(engagementId),
				listTrustRelationships(engagementId),
				listPivotTunnels(engagementId)
			]);
			host = h;
			services = svc;
			checklists = cls;
			notes = hostNotes;
			attachments = hostAttachments;
			allHosts = hosts;
			allTrust = trust;
			allPivots = pivots;
			labelDraft = h.label;
			hostnameDraft = h.hostname ?? '';
			osDraft = h.os ?? '';
			osFamilyDraft = h.os_family ?? '';
			criticalityDraft = h.criticality ?? '';
			statusDraft = h.status;
			notesDraft = h.general_info_md;
			loginNotesDraft = h.login_notes_md;
			isFootholdDraft = h.is_foothold;
			isPivotDraft = h.is_pivot;
		} catch {
			error = 'Failed to load host.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	// General/login-notes/foothold all replace the same full host record --
	// the backend has one PUT, not a field-level PATCH -- so every "Save"
	// button on this page shares this one call, differing only in which
	// error message to show if it fails.
	async function saveHost(failMessage: string) {
		try {
			host = await updateHost(hostId, {
				label: labelDraft,
				hostname: hostnameDraft || null,
				os: osDraft || null,
				os_family: osFamilyDraft || null,
				criticality: criticalityDraft || null,
				status: statusDraft,
				general_info_md: notesDraft,
				login_notes_md: loginNotesDraft,
				is_foothold: isFootholdDraft,
				is_pivot: isPivotDraft
			});
			error = '';
		} catch {
			error = failMessage;
		}
	}

	async function handleLogAccess(e: SubmitEvent) {
		e.preventDefault();
		if (!accessOtherHostId) return;
		try {
			const payload =
				accessDirection === 'from'
					? { from_host_id: accessOtherHostId, to_host_id: hostId, kind: accessKind, note: accessNote || null }
					: { from_host_id: hostId, to_host_id: accessOtherHostId, kind: accessKind, note: accessNote || null };
			await createTrustRelationship(engagementId, payload);
			allTrust = await listTrustRelationships(engagementId);
			accessOtherHostId = '';
			accessNote = '';
			error = '';
		} catch {
			error = 'Failed to log access.';
		}
	}

	async function handleRemoveAccess(id: string) {
		try {
			await deleteTrustRelationship(id);
			allTrust = allTrust.filter((t) => t.id !== id);
		} catch {
			error = 'Failed to remove access entry.';
		}
	}

	async function handleAddPivot(e: SubmitEvent) {
		e.preventDefault();
		try {
			const pivot = await createPivotTunnel(engagementId, {
				from_host_id: hostId,
				to_host_id: newPivotToHostId || null,
				method: newPivotMethod,
				local_port: newPivotLocalPort === '' ? null : Number(newPivotLocalPort),
				remote_target: newPivotRemoteTarget || null,
				notes_md: newPivotNotes
			});
			allPivots = [...allPivots, pivot];
			newPivotToHostId = '';
			newPivotLocalPort = '';
			newPivotRemoteTarget = '';
			newPivotNotes = '';
			error = '';
		} catch {
			error = 'Failed to log pivot tunnel.';
		}
	}

	async function handleRemovePivot(id: string) {
		try {
			await deletePivotTunnel(id);
			allPivots = allPivots.filter((p) => p.id !== id);
		} catch {
			error = 'Failed to remove pivot tunnel.';
		}
	}

	async function handleAddIp() {
		if (!newIp.trim()) return;
		try {
			host = await addAddress(hostId, newIp.trim());
			newIp = '';
			error = '';
		} catch {
			error = 'Failed to add IP address (check the format).';
		}
	}

	async function handleRemoveIp(addressId: string) {
		try {
			await removeAddress(hostId, addressId);
			if (host) {
				host = { ...host, addresses: host.addresses.filter((a) => a.id !== addressId) };
			}
		} catch {
			error = 'Failed to remove address.';
		}
	}

	async function handleAddTag() {
		if (!newTagName.trim()) return;
		try {
			host = await addTag(hostId, newTagName.trim());
			newTagName = '';
			error = '';
		} catch {
			error = 'Failed to add tag.';
		}
	}

	async function handleRemoveTag(tagId: string) {
		try {
			await removeTag(hostId, tagId);
			if (host) {
				host = { ...host, tags: host.tags.filter((t) => t.id !== tagId) };
			}
		} catch {
			error = 'Failed to remove tag.';
		}
	}

	async function handleAddService(e: SubmitEvent) {
		e.preventDefault();
		if (newPort === '') return;
		try {
			const service = await createService(hostId, {
				port: Number(newPort),
				protocol: newProtocol,
				name: newServiceName || null,
				display_name: newDisplayName || null,
				version: newVersion || null
			});
			services = [...services, service];
			newPort = '';
			newServiceName = '';
			newDisplayName = '';
			newVersion = '';
			error = '';
			// A matching service type may have auto-instantiated a checklist.
			checklists = await listHostChecklists(hostId);
		} catch {
			error = 'Failed to add service.';
		}
	}

	async function handleDeleteService(serviceId: string) {
		try {
			await deleteService(hostId, serviceId);
			services = services.filter((s) => s.id !== serviceId);
			// Removing the last service of a type de-instantiates its checklist.
			checklists = await listHostChecklists(hostId);
		} catch {
			error = 'Failed to remove service.';
		}
	}

</script>

<main>
	{#if loading}
		<p>Loading…</p>
	{:else if !host}
		<p class="error">{error || 'Host not found.'}</p>
	{:else}
		<p><a href={`/engagements/${engagementId}/hosts`}>&larr; All hosts</a></p>
		<h1>{host.label}</h1>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		<nav class="tabs">
			<button class:active={activeTab === 'general'} onclick={() => (activeTab = 'general')}>
				General &amp; access ({incomingAccess.length})
			</button>
			<button class:active={activeTab === 'services'} onclick={() => (activeTab = 'services')}>
				Services ({services.length})
			</button>
			<button class:active={activeTab === 'checklists'} onclick={() => (activeTab = 'checklists')}>
				Checklists ({checklists.length})
			</button>
			<button class:active={activeTab === 'notes'} onclick={() => (activeTab = 'notes')}>
				Notes ({notes.length})
			</button>
			<button class:active={activeTab === 'attachments'} onclick={() => (activeTab = 'attachments')}>
				Attachments ({attachments.length})
			</button>
		</nav>

		{#if activeTab === 'general'}
			<section class="general-access-grid">
				<div class="general-col">
					<div class="grid">
						<label>
							Label
							<input bind:value={labelDraft} />
						</label>
						<label>
							Hostname
							<input bind:value={hostnameDraft} />
						</label>
						<label>
							OS
							<input bind:value={osDraft} />
						</label>
						<label>
							OS family
							<input bind:value={osFamilyDraft} />
						</label>
						<label>
							Criticality
							<input bind:value={criticalityDraft} />
						</label>
						<label>
							Status
							<select bind:value={statusDraft}>
								<option value="discovered">discovered</option>
								<option value="enumerating">enumerating</option>
								<option value="exploited">exploited</option>
								<option value="owned">owned</option>
								<option value="cleared">cleared</option>
							</select>
						</label>
					</div>
					<label>
						General notes
						<textarea bind:value={notesDraft} rows="6"></textarea>
					</label>
					<button onclick={() => saveHost('Failed to save host.')}>Save</button>

					<h2>IP addresses</h2>
					<ul class="chips">
						{#each host.addresses as addr (addr.id)}
							<li class="chip">
								{addr.ip}{addr.is_primary ? ' (primary)' : ''}
								<button onclick={() => handleRemoveIp(addr.id)}>&times;</button>
							</li>
						{/each}
					</ul>
					<div class="inline-form">
						<input bind:value={newIp} placeholder="10.10.10.7" />
						<button onclick={handleAddIp}>Add</button>
					</div>

					<h2>Tags</h2>
					<ul class="chips">
						{#each host.tags as tag (tag.id)}
							<li class="chip">
								{tag.name}
								<button onclick={() => handleRemoveTag(tag.id)}>&times;</button>
							</li>
						{/each}
					</ul>
					<div class="inline-form">
						<input bind:value={newTagName} placeholder="new-tag" />
						<button onclick={handleAddTag}>Add</button>
					</div>
				</div>

				<div class="access-col">
					<h2>Accessible from</h2>
					{#if incomingAccess.length === 0}
						<p class="muted">
							No recorded access to this host yet — log how you got in below, or log it as a
							pivot target from another host's access section.
						</p>
					{:else}
						<ul class="access-list">
							{#each incomingAccess as t (t.id)}
								<li>
									Accessible from
									<a href={`/engagements/${engagementId}/hosts/${t.from_host_id}`}>{t.from_host_label}</a>
									<span class="muted">({t.kind}){t.note ? ` — ${t.note}` : ''}</span>
									<button onclick={() => handleRemoveAccess(t.id)}>&times;</button>
								</li>
							{/each}
						</ul>
					{/if}

					<h3>Log access</h3>
					<form onsubmit={handleLogAccess} class="inline-form">
						<select bind:value={accessDirection}>
							<option value="from">This host is accessible from…</option>
							<option value="to">This host has access to…</option>
						</select>
						<select bind:value={accessOtherHostId}>
							<option value="" disabled selected>Other host…</option>
							{#each allHosts.filter((h) => h.id !== hostId) as h (h.id)}
								<option value={h.id}>{h.label}</option>
							{/each}
						</select>
						<select bind:value={accessKind}>
							<option value="domain_trust">domain trust</option>
							<option value="admin_of">admin of</option>
							<option value="shares_creds">shares creds</option>
							<option value="session">session</option>
						</select>
						<input bind:value={accessNote} placeholder="note (optional)" />
						<button type="submit">Log</button>
					</form>

					<h2>Pivots &amp; tunnels</h2>
					{#if pivotsFromHost.length === 0}
						<p class="muted">No pivots logged from this host yet.</p>
					{:else}
						<ul class="access-list">
							{#each pivotsFromHost as p (p.id)}
								<li>
									{p.method}
									{#if p.to_host_id}
										&rarr;
										<a href={`/engagements/${engagementId}/hosts/${p.to_host_id}`}>{p.to_host_label}</a>
									{:else}
										&rarr; <span class="muted">{p.remote_target ?? 'network segment'}</span>
									{/if}
									{#if p.local_port}
										<span class="muted">(local port {p.local_port})</span>
									{/if}
									{#if p.notes_md}
										<span class="muted">— {p.notes_md}</span>
									{/if}
									<button onclick={() => handleRemovePivot(p.id)}>&times;</button>
								</li>
							{/each}
						</ul>
					{/if}
					<h3>Log pivot</h3>
					<form onsubmit={handleAddPivot} class="inline-form">
						<select bind:value={newPivotMethod}>
							{#each PIVOT_METHODS as m (m)}
								<option value={m}>{m}</option>
							{/each}
						</select>
						<select bind:value={newPivotToHostId}>
							<option value="">(subnet / no specific host)</option>
							{#each allHosts.filter((h) => h.id !== hostId) as h (h.id)}
								<option value={h.id}>{h.label}</option>
							{/each}
						</select>
						<input
							type="number"
							min="0"
							max="65535"
							bind:value={newPivotLocalPort}
							placeholder="local port"
						/>
						<input bind:value={newPivotRemoteTarget} placeholder="remote target (e.g. CIDR)" />
						<input bind:value={newPivotNotes} placeholder="notes (optional)" />
						<button type="submit">Log pivot</button>
					</form>

					<label class="foothold-toggle">
						<input type="checkbox" bind:checked={isFootholdDraft} />
						Mark this host as the foothold / initial access point
					</label>
					<label class="foothold-toggle">
						<input type="checkbox" bind:checked={isPivotDraft} />
						Mark this host as a pivot point
					</label>
					<button onclick={() => saveHost('Failed to save foothold/pivot status.')}>Save</button>

					<h2>Login procedure notes</h2>
					<label>
						<textarea bind:value={loginNotesDraft} rows="8"></textarea>
					</label>
					<button onclick={() => saveHost('Failed to save login procedure notes.')}>Save</button>
				</div>
			</section>
		{:else if activeTab === 'services'}
			<section>
				<table>
					<thead>
						<tr>
							<th>Port</th>
							<th>Proto</th>
							<th>Service</th>
							<th>Display name</th>
							<th>Version</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each services as service (service.id)}
							<tr>
								<td>{service.port}</td>
								<td>{service.protocol}</td>
								<td>{service.name ?? ''}</td>
								<td>{service.display_name ?? ''}</td>
								<td>{service.version ?? ''}</td>
								<td><button onclick={() => handleDeleteService(service.id)}>Remove</button></td>
							</tr>
						{/each}
					</tbody>
				</table>
				<form onsubmit={handleAddService} class="inline-form">
					<input type="number" min="0" max="65535" bind:value={newPort} placeholder="port" required />
					<select bind:value={newProtocol}>
						<option value="tcp">tcp</option>
						<option value="udp">udp</option>
					</select>
					<select bind:value={newServiceName}>
						<option value="" disabled selected>Service…</option>
						{#each SERVICE_NAMES as svc (svc)}
							<option value={svc}>{svc}</option>
						{/each}
					</select>
					<input bind:value={newDisplayName} placeholder="display name (optional)" />
					<input bind:value={newVersion} placeholder="version" />
					<button type="submit">Add service</button>
				</form>
			</section>
		{:else if activeTab === 'checklists'}
			<section>
				{#if checklists.length === 0}
					<p class="muted">No checklists yet — add a matching service on the Services tab to get one.</p>
				{:else}
					{#each checklists as checklist (checklist.id)}
						<ChecklistPanel
							{checklist}
							onchange={(updated) => {
								checklists = checklists.map((c) => (c.id === updated.id ? updated : c));
							}}
						/>
					{/each}
				{/if}
			</section>
		{:else if activeTab === 'notes'}
			<section>
				<NoteEditor {engagementId} subjectType="host" subjectId={hostId} bind:notes />
			</section>
		{:else}
			<section>
				<AttachmentGallery {engagementId} subjectType="host" subjectId={hostId} bind:attachments />
			</section>
		{/if}
	{/if}
</main>

<style>
	.error {
		color: var(--error);
	}
	.tabs {
		display: flex;
		gap: 0.5rem;
		margin: 1rem 0;
		border-bottom: 1px solid var(--border);
	}
	.tabs button {
		background: none;
		border: none;
		padding: 0.5rem 0.75rem;
		cursor: pointer;
		border-bottom: 2px solid transparent;
	}
	.tabs button.active {
		border-bottom-color: var(--accent);
		font-weight: 600;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	.general-access-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 2rem;
		align-items: start;
	}
	.access-col {
		border-left: 1px solid var(--border);
		padding-left: 2rem;
	}
	@media (max-width: 900px) {
		.general-access-grid {
			grid-template-columns: 1fr;
		}
		.access-col {
			border-left: none;
			padding-left: 0;
			border-top: 1px solid var(--border);
			padding-top: 1rem;
		}
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
	}
	.foothold-toggle {
		flex-direction: row;
		align-items: center;
		gap: 0.4rem;
		margin-bottom: 0.75rem;
	}
	textarea {
		width: 100%;
		font-family: inherit;
	}
	.chips {
		list-style: none;
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		padding: 0;
		margin: 0.5rem 0;
	}
	.chip {
		background: var(--surface-2);
		border-radius: 999px;
		padding: 0.15rem 0.4rem 0.15rem 0.7rem;
		display: flex;
		align-items: center;
		gap: 0.3rem;
		font-size: 0.85rem;
	}
	.chip button {
		background: none;
		border: none;
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
	}
	.inline-form {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
		margin-bottom: 1rem;
	}
	table {
		border-collapse: collapse;
		width: 100%;
		margin-bottom: 1rem;
	}
	th,
	td {
		text-align: left;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--border);
	}
	.muted {
		color: var(--text-muted);
	}
	.access-list {
		list-style: none;
		padding: 0;
		margin: 0 0 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.access-list li {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.access-list button {
		background: none;
		border: none;
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
	}
</style>
