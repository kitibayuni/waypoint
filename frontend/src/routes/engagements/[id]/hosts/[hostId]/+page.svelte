<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import {
		getHost,
		updateHost,
		addAddress,
		removeAddress,
		addTag,
		removeTag
	} from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { listServices, createService, deleteService } from '$lib/api/services';
	import type { Service } from '$lib/api/services';
	import {
		listObservationTypes,
		listObservations,
		createObservation,
		updateObservation,
		deleteObservation
	} from '$lib/api/observations';
	import type { ObservationType, Observation } from '$lib/api/observations';
	import ObservationChip from '$lib/components/ObservationChip.svelte';

	const engagementId = $page.params.id as string;
	const hostId = $page.params.hostId as string;

	let host = $state<Host | null>(null);
	let services = $state<Service[]>([]);
	let loading = $state(true);
	let error = $state('');
	let activeTab = $state<'general' | 'services' | 'observations'>('general');

	let labelDraft = $state('');
	let hostnameDraft = $state('');
	let osDraft = $state('');
	let osFamilyDraft = $state('');
	let criticalityDraft = $state('');
	let statusDraft = $state('discovered');
	let notesDraft = $state('');

	let newIp = $state('');
	let newTagName = $state('');

	let newPort = $state<number | ''>('');
	let newProtocol = $state('tcp');
	let newServiceName = $state('');
	let newProduct = $state('');
	let newVersion = $state('');

	let observationTypes = $state<ObservationType[]>([]);
	let observations = $state<Observation[]>([]);
	let newObservationTypeId = $state('');
	let newObservationEvidence = $state('');
	let expandedObservationId = $state<string | null>(null);
	let editStatusDraft = $state('suspected');
	let editEvidenceDraft = $state('');
	let editSeverityDraft = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			const [h, svc, obsTypes, obs] = await Promise.all([
				getHost(hostId),
				listServices(hostId),
				listObservationTypes(),
				listObservations(hostId)
			]);
			host = h;
			services = svc;
			observationTypes = obsTypes;
			observations = obs;
			labelDraft = h.label;
			hostnameDraft = h.hostname ?? '';
			osDraft = h.os ?? '';
			osFamilyDraft = h.os_family ?? '';
			criticalityDraft = h.criticality ?? '';
			statusDraft = h.status;
			notesDraft = h.general_info_md;
		} catch {
			error = 'Failed to load host.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function saveGeneral() {
		try {
			host = await updateHost(hostId, {
				label: labelDraft,
				hostname: hostnameDraft || null,
				os: osDraft || null,
				os_family: osFamilyDraft || null,
				criticality: criticalityDraft || null,
				status: statusDraft,
				general_info_md: notesDraft
			});
			error = '';
		} catch {
			error = 'Failed to save host.';
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
				product: newProduct || null,
				version: newVersion || null
			});
			services = [...services, service];
			newPort = '';
			newServiceName = '';
			newProduct = '';
			newVersion = '';
			error = '';
		} catch {
			error = 'Failed to add service.';
		}
	}

	async function handleDeleteService(serviceId: string) {
		try {
			await deleteService(hostId, serviceId);
			services = services.filter((s) => s.id !== serviceId);
		} catch {
			error = 'Failed to remove service.';
		}
	}

	async function handleAttachObservation() {
		if (!newObservationTypeId) return;
		try {
			const observation = await createObservation(hostId, {
				observation_type_id: newObservationTypeId,
				evidence_md: newObservationEvidence
			});
			observations = [...observations, observation];
			newObservationTypeId = '';
			newObservationEvidence = '';
			error = '';
		} catch {
			error = 'Failed to attach observation.';
		}
	}

	function toggleExpand(observation: Observation) {
		if (expandedObservationId === observation.id) {
			expandedObservationId = null;
			return;
		}
		expandedObservationId = observation.id;
		editStatusDraft = observation.status;
		editEvidenceDraft = observation.evidence_md;
		editSeverityDraft = observation.severity_override ?? '';
	}

	async function saveObservationEdit(observationId: string) {
		try {
			const updated = await updateObservation(observationId, {
				status: editStatusDraft,
				evidence_md: editEvidenceDraft,
				severity_override: editSeverityDraft || null
			});
			observations = observations.map((o) => (o.id === observationId ? updated : o));
			expandedObservationId = null;
			error = '';
		} catch {
			error = 'Failed to update observation.';
		}
	}

	async function handleDeleteObservation(observationId: string) {
		try {
			await deleteObservation(observationId);
			observations = observations.filter((o) => o.id !== observationId);
			expandedObservationId = null;
		} catch {
			error = 'Failed to remove observation.';
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
				General
			</button>
			<button class:active={activeTab === 'services'} onclick={() => (activeTab = 'services')}>
				Services ({services.length})
			</button>
			<button
				class:active={activeTab === 'observations'}
				onclick={() => (activeTab = 'observations')}
			>
				Observations ({observations.length})
			</button>
		</nav>

		{#if activeTab === 'general'}
			<section>
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
				<button onclick={saveGeneral}>Save</button>

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
			</section>
		{:else if activeTab === 'services'}
			<section>
				<table>
					<thead>
						<tr>
							<th>Port</th>
							<th>Proto</th>
							<th>Name</th>
							<th>Product</th>
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
								<td>{service.product ?? ''}</td>
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
					<input bind:value={newServiceName} placeholder="name (e.g. smb)" />
					<input bind:value={newProduct} placeholder="product" />
					<input bind:value={newVersion} placeholder="version" />
					<button type="submit">Add service</button>
				</form>
			</section>
		{:else}
			<section>
				<div class="inline-form">
					<select bind:value={newObservationTypeId}>
						<option value="" disabled selected>Select an observation…</option>
						{#each observationTypes as ot (ot.id)}
							<option value={ot.id}>[{ot.category}] {ot.title}</option>
						{/each}
					</select>
					<input bind:value={newObservationEvidence} placeholder="Evidence (optional)" />
					<button onclick={handleAttachObservation}>Attach</button>
				</div>

				<div class="observation-list">
					{#each observations as observation (observation.id)}
						<div class="observation-item">
							<ObservationChip {observation} onclick={() => toggleExpand(observation)} />
							{#if expandedObservationId === observation.id}
								<div class="observation-edit">
									<label>
										Status
										<select bind:value={editStatusDraft}>
											<option value="suspected">suspected</option>
											<option value="confirmed">confirmed</option>
											<option value="remediated">remediated</option>
											<option value="false_positive">false positive</option>
										</select>
									</label>
									<label>
										Severity override
										<input bind:value={editSeverityDraft} placeholder={observation.default_severity} />
									</label>
									<label>
										Evidence
										<textarea bind:value={editEvidenceDraft} rows="4"></textarea>
									</label>
									<div class="observation-edit-actions">
										<button onclick={() => saveObservationEdit(observation.id)}>Save</button>
										<button onclick={() => handleDeleteObservation(observation.id)}>Remove</button>
									</div>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			</section>
		{/if}
	{/if}
</main>

<style>
	.error {
		color: #c0392b;
	}
	.tabs {
		display: flex;
		gap: 0.5rem;
		margin: 1rem 0;
		border-bottom: 1px solid #ddd;
	}
	.tabs button {
		background: none;
		border: none;
		padding: 0.5rem 0.75rem;
		cursor: pointer;
		border-bottom: 2px solid transparent;
	}
	.tabs button.active {
		border-bottom-color: #333;
		font-weight: 600;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
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
		background: #eee;
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
		border-bottom: 1px solid #ddd;
	}
	.observation-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-top: 1rem;
	}
	.observation-item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
	}
	.observation-edit {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.6rem;
		width: 100%;
		max-width: 28rem;
	}
	.observation-edit textarea {
		font-family: inherit;
	}
	.observation-edit-actions {
		display: flex;
		gap: 0.5rem;
	}
</style>
