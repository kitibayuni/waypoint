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
	import CvssCalculator from '$lib/components/CvssCalculator.svelte';
	import type { CvssResult } from '$lib/cvss';
	import { listMitreTechniques } from '$lib/api/mitre';
	import type { MitreTechnique } from '$lib/api/mitre';
	import { getFindingHistory } from '$lib/api/audit';
	import type { FindingHistoryEntry } from '$lib/api/audit';

	const engagementId = $page.params.id as string;
	const findingId = $page.params.findingId as string;

	let finding = $state<Finding | null>(null);
	let hosts = $state<Host[]>([]);
	let notes = $state<Note[]>([]);
	let attachments = $state<Attachment[]>([]);
	let mitreTechniques = $state<MitreTechnique[]>([]);
	let history = $state<FindingHistoryEntry[]>([]);
	let loading = $state(true);
	let error = $state('');
	let activeTab = $state<'details' | 'notes' | 'attachments' | 'history'>('details');
	let showCvssCalc = $state(false);

	let titleDraft = $state('');
	let cveDraft = $state('');
	let cvssVectorDraft = $state('');
	let cvssScoreDraft = $state<number | ''>('');
	let severityDraft = $state('');
	let statusDraft = $state('open');
	let descriptionDraft = $state('');
	let remediationDraft = $state('');
	let pocDraft = $state('');
	let mitreIdsDraft = $state('');
	let affectedHostIds = $state<string[]>([]);

	function mitreName(id: string): string | null {
		return mitreTechniques.find((t) => t.id === id)?.name ?? null;
	}

	async function load() {
		loading = true;
		error = '';
		try {
			const [f, hostList, noteList, attachmentList, techniques, historyList] = await Promise.all([
				getFinding(findingId),
				listHosts(engagementId),
				listNotes(engagementId, 'finding', findingId),
				listAttachments(engagementId, 'finding', findingId),
				listMitreTechniques(),
				getFindingHistory(findingId)
			]);
			finding = f;
			hosts = hostList;
			notes = noteList;
			attachments = attachmentList;
			mitreTechniques = techniques;
			history = historyList;
			titleDraft = f.title;
			cveDraft = f.cve ?? '';
			cvssVectorDraft = f.cvss_vector ?? '';
			cvssScoreDraft = f.cvss_score ?? '';
			severityDraft = f.severity ?? '';
			statusDraft = f.status;
			descriptionDraft = f.description_md;
			remediationDraft = f.remediation_md;
			pocDraft = f.poc_md;
			mitreIdsDraft = f.mitre_technique_ids.join(', ');
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
			const mitreIds = mitreIdsDraft
				.split(',')
				.map((s) => s.trim())
				.filter(Boolean);
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
				mitre_technique_ids: mitreIds,
				affected_host_ids: affectedHostIds
			});
			history = await getFindingHistory(findingId);
			error = '';
		} catch {
			error = 'Failed to save finding.';
		}
	}

	function handleApplyCvss(result: CvssResult) {
		cvssVectorDraft = result.vector;
		cvssScoreDraft = result.score;
		if (result.severity !== 'none') {
			severityDraft = result.severity;
		}
		showCvssCalc = false;
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
			<button class:active={activeTab === 'history'} onclick={() => (activeTab = 'history')}>
				History ({history.length})
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

				<button type="button" onclick={() => (showCvssCalc = !showCvssCalc)}>
					{showCvssCalc ? 'Hide' : 'Open'} CVSS calculator
				</button>
				{#if showCvssCalc}
					<CvssCalculator onApply={handleApplyCvss} />
				{/if}

				<label>
					MITRE ATT&amp;CK technique IDs (comma-separated)
					<input bind:value={mitreIdsDraft} placeholder="T1557.001, T1558.003" />
				</label>
				{#if mitreIdsDraft.trim()}
					<ul class="mitre-list">
						{#each mitreIdsDraft.split(',').map((s) => s.trim()).filter(Boolean) as mid (mid)}
							<li>
								<code>{mid}</code>
								{#if mitreName(mid)}&mdash; {mitreName(mid)}{/if}
							</li>
						{/each}
					</ul>
				{/if}

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
		{:else if activeTab === 'attachments'}
			<section>
				<AttachmentGallery {engagementId} subjectType="finding" subjectId={findingId} bind:attachments />
			</section>
		{:else}
			<section>
				{#if history.length === 0}
					<p>No history yet.</p>
				{:else}
					<ul class="history-list">
						{#each history as entry (entry.id)}
							<li>
								<div class="history-head">
									<strong>{entry.action}</strong>
									<span>{entry.actor_email ?? 'unknown'}</span>
									<time>{new Date(entry.at).toLocaleString()}</time>
								</div>
								<div class="history-diff">
									{#each Array.from(new Set([...Object.keys(entry.before ?? {}), ...Object.keys(entry.after ?? {})])) as key (key)}
										{@const beforeVal = entry.before ? entry.before[key] : undefined}
										{@const afterVal = entry.after ? entry.after[key] : undefined}
										{#if JSON.stringify(beforeVal) !== JSON.stringify(afterVal)}
											<div class="diff-row">
												<span class="diff-key">{key}</span>
												<span class="diff-before">{JSON.stringify(beforeVal)}</span>
												<span class="diff-arrow">&rarr;</span>
												<span class="diff-after">{JSON.stringify(afterVal)}</span>
											</div>
										{/if}
									{/each}
								</div>
							</li>
						{/each}
					</ul>
				{/if}
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
	.mitre-list {
		list-style: none;
		padding: 0;
		margin: 0 0 0.75rem;
		font-size: 0.85rem;
		color: #555;
	}
	.history-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.history-list > li {
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.6rem 0.75rem;
	}
	.history-head {
		display: flex;
		gap: 0.75rem;
		align-items: baseline;
		font-size: 0.85rem;
		color: #555;
		margin-bottom: 0.4rem;
	}
	.history-head strong {
		text-transform: uppercase;
		color: #333;
	}
	.history-diff {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		font-size: 0.8rem;
	}
	.diff-row {
		display: grid;
		grid-template-columns: 8rem 1fr auto 1fr;
		gap: 0.4rem;
		align-items: baseline;
	}
	.diff-key {
		font-weight: 600;
		color: #333;
	}
	.diff-before {
		color: #c0392b;
		text-decoration: line-through;
		word-break: break-word;
	}
	.diff-after {
		color: #0ca30c;
		word-break: break-word;
	}
	.diff-arrow {
		color: #888;
	}
</style>
