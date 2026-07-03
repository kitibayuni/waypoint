<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { listFindings, createFinding } from '$lib/api/findings';
	import type { Finding } from '$lib/api/findings';

	const engagementId = $page.params.id as string;

	let findings = $state<Finding[]>([]);
	let loading = $state(true);
	let error = $state('');
	let newTitle = $state('');

	async function load() {
		loading = true;
		error = '';
		try {
			findings = await listFindings(engagementId);
		} catch {
			error = 'Failed to load findings.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function handleCreate(e: SubmitEvent) {
		e.preventDefault();
		if (!newTitle.trim()) return;
		try {
			const finding = await createFinding(engagementId, { title: newTitle });
			goto(`/engagements/${engagementId}/findings/${finding.id}`);
		} catch {
			error = 'Failed to create finding.';
		}
	}

	function severityClass(severity: string | null): string {
		return severity ? `severity-${severity}` : '';
	}
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Findings</h1>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<form onsubmit={handleCreate} class="inline-form">
		<input bind:value={newTitle} placeholder="New finding title" required />
		<button type="submit">Create finding</button>
	</form>

	{#if loading}
		<p>Loading…</p>
	{:else if findings.length === 0}
		<p>No findings yet.</p>
	{:else}
		<table>
			<thead>
				<tr>
					<th>Title</th>
					<th>Severity</th>
					<th>Status</th>
					<th>CVE</th>
					<th>Affected hosts</th>
				</tr>
			</thead>
			<tbody>
				{#each findings as finding (finding.id)}
					<tr onclick={() => goto(`/engagements/${engagementId}/findings/${finding.id}`)}>
						<td>{finding.title}</td>
						<td><span class={severityClass(finding.severity)}>{finding.severity ?? ''}</span></td>
						<td>{finding.status}</td>
						<td>{finding.cve ?? ''}</td>
						<td>{finding.affected_hosts.map((h) => h.label).join(', ')}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</main>

<style>
	.error {
		color: #c0392b;
	}
	.inline-form {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		max-width: 32rem;
	}
	.inline-form input {
		flex: 1;
	}
	table {
		border-collapse: collapse;
		width: 100%;
	}
	th,
	td {
		text-align: left;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid #ddd;
	}
	tr {
		cursor: pointer;
	}
	tr:hover {
		background: #f7f7f7;
	}
	.severity-critical {
		color: #922b21;
		font-weight: 600;
	}
	.severity-high {
		color: #c0392b;
	}
	.severity-medium {
		color: #d4a017;
	}
	.severity-low {
		color: #2a7a2a;
	}
</style>
