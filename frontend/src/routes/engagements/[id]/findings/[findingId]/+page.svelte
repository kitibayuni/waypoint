<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { getFinding, updateFinding, deleteFinding } from '$lib/api/findings';
	import type { Finding } from '$lib/api/findings';
	import { listHosts } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import { listNotes } from '$lib/api/notes';
	import type { Note } from '$lib/api/notes';
	import NoteEditor from '$lib/components/NoteEditor.svelte';
	import { listAttachments } from '$lib/api/attachments';
	import type { Attachment } from '$lib/api/attachments';
	import AttachmentGallery from '$lib/components/AttachmentGallery.svelte';

	const engagementId = $page.params.id as string;
	const findingId = $page.params.findingId as string;

	let finding = $state<Finding | null>(null);
	let hosts = $state<Host[]>([]);
	let notes = $state<Note[]>([]);
	let attachments = $state<Attachment[]>([]);
	let loading = $state(true);
	let error = $state('');
	let activeTab = $state<'details' | 'notes' | 'attachments'>('details');

	let titleDraft = $state('');
	let cveDraft = $state('');
	let cvssVectorDraft = $state('');
	let cvssScoreDraft = $state<number | ''>('');
	let severityDraft = $state('');
	let statusDraft = $state('open');
	let descriptionDraft = $state('');
	let remediationDraft = $state('');
	let pocDraft = $state('');
	let affectedHostIds = $state<string[]>([]);

	async function load() {
		loading = true;
		error = '';
		try {
			const [f, hostList, noteList, attachmentList] = await Promise.all([
				getFinding(findingId),
				listHosts(engagementId),
				listNotes(engagementId, 'finding', findingId),
				listAttachments(engagementId, 'finding', findingId)
			]);
			finding = f;
			hosts = hostList;
			notes = noteList;
			attachments = attachmentList;
			titleDraft = f.title;
			cveDraft = f.cve ?? '';
			cvssVectorDraft = f.cvss_vector ?? '';
			cvssScoreDraft = f.cvss_score ?? '';
			severityDraft = f.severity ?? '';
			statusDraft = f.status;
			descriptionDraft = f.description_md;
			remediationDraft = f.remediation_md;
			pocDraft = f.poc_md;
			affectedHostIds = f.affected_hosts.map((h) => h.id);
		} catch {
			error = 'Failed to load finding.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function handleSave() {
		try {
			finding = await updateFinding(findingId, {
				title: titleDraft,
				cve: cveDraft || null,
				cvss_vector: cvssVectorDraft || null,
				cvss_score: cvssScoreDraft === '' ? null : Number(cvssScoreDraft),
				severity: severityDraft || null,
				status: statusDraft,
				description_md: descriptionDraft,
				remediation_md: remediationDraft,
				poc_md: pocDraft,
				affected_host_ids: affectedHostIds
			});
			error = '';
		} catch {
			error = 'Failed to save finding.';
		}
	}

	async function handleDelete() {
		try {
			await deleteFinding(findingId);
			goto(`/engagements/${engagementId}/findings`);
		} catch {
			error = 'Failed to remove finding.';
		}
	}

	function toggleHost(hostId: string) {
		if (affectedHostIds.includes(hostId)) {
			affectedHostIds = affectedHostIds.filter((id) => id !== hostId);
		} else {
			affectedHostIds = [...affectedHostIds, hostId];
		}
	}
</script>

<main>
	{#if loading}
		<p>Loading…</p>
	{:else if !finding}
		<p class="error">{error || 'Finding not found.'}</p>
	{:else}
		<p><a href={`/engagements/${engagementId}/findings`}>&larr; All findings</a></p>
		<h1>{finding.title}</h1>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		<nav class="tabs">
			<button class:active={activeTab === 'details'} onclick={() => (activeTab = 'details')}>
				Details
			</button>
			<button class:active={activeTab === 'notes'} onclick={() => (activeTab = 'notes')}>
				Notes ({notes.length})
			</button>
			<button class:active={activeTab === 'attachments'} onclick={() => (activeTab = 'attachments')}>
				Attachments ({attachments.length})
			</button>
		</nav>

		{#if activeTab === 'details'}
			<section>
				<div class="grid">
					<label>
						Title
						<input bind:value={titleDraft} />
					</label>
					<label>
						CVE
						<input bind:value={cveDraft} placeholder="CVE-2024-XXXXX" />
					</label>
					<label>
						CVSS vector
						<input bind:value={cvssVectorDraft} placeholder="CVSS:3.1/AV:N/..." />
					</label>
					<label>
						CVSS score
						<input type="number" min="0" max="10" step="0.1" bind:value={cvssScoreDraft} />
					</label>
					<label>
						Severity
						<select bind:value={severityDraft}>
							<option value="">(none)</option>
							<option value="critical">critical</option>
							<option value="high">high</option>
							<option value="medium">medium</option>
							<option value="low">low</option>
						</select>
					</label>
					<label>
						Status
						<select bind:value={statusDraft}>
							<option value="open">open</option>
							<option value="triaged">triaged</option>
							<option value="accepted_risk">accepted risk</option>
							<option value="fixed">fixed</option>
						</select>
					</label>
				</div>

				<label>
					Description
					<textarea bind:value={descriptionDraft} rows="6"></textarea>
				</label>
				<label>
					Remediation
					<textarea bind:value={remediationDraft} rows="6"></textarea>
				</label>
				<label>
					Proof of concept
					<textarea bind:value={pocDraft} rows="6"></textarea>
				</label>

				<h2>Affected hosts</h2>
				<div class="host-checkboxes">
					{#each hosts as host (host.id)}
						<label class="checkbox">
							<input
								type="checkbox"
								checked={affectedHostIds.includes(host.id)}
								onchange={() => toggleHost(host.id)}
							/>
							{host.label}
						</label>
					{/each}
				</div>

				<div class="actions">
					<button onclick={handleSave}>Save</button>
					<button onclick={handleDelete}>Delete finding</button>
				</div>
			</section>
		{:else if activeTab === 'notes'}
			<section>
				<NoteEditor {engagementId} subjectType="finding" subjectId={findingId} bind:notes />
			</section>
		{:else}
			<section>
				<AttachmentGallery {engagementId} subjectType="finding" subjectId={findingId} bind:attachments />
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
		margin-bottom: 0.75rem;
	}
	textarea {
		width: 100%;
		font-family: inherit;
	}
	.host-checkboxes {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}
	.checkbox {
		flex-direction: row;
		align-items: center;
		gap: 0.3rem;
	}
	.actions {
		display: flex;
		gap: 0.5rem;
	}
</style>
