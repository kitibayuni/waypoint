<script lang="ts">
	import { page } from '$app/stores';
	import { previewImport, commitImport } from '$lib/api/import';
	import type { ImportPreview, ImportResult } from '$lib/api/import';

	const engagementId = $page.params.id as string;

	let source = $state('nmap');
	let fileInput = $state<HTMLInputElement | undefined>();
	let selectedFile = $state<File | null>(null);
	let preview = $state<ImportPreview | null>(null);
	let result = $state<ImportResult | null>(null);
	let loading = $state(false);
	let error = $state('');

	function handleFileChange() {
		selectedFile = fileInput?.files?.[0] ?? null;
		preview = null;
		result = null;
		error = '';
	}

	async function handlePreview() {
		if (!selectedFile) return;
		loading = true;
		error = '';
		result = null;
		try {
			preview = await previewImport(source, engagementId, selectedFile);
		} catch {
			error = 'Failed to parse the uploaded file. Check the format matches the selected source.';
			preview = null;
		} finally {
			loading = false;
		}
	}

	async function handleCommit() {
		if (!selectedFile) return;
		loading = true;
		error = '';
		try {
			result = await commitImport(source, engagementId, selectedFile);
			preview = null;
			selectedFile = null;
			if (fileInput) fileInput.value = '';
		} catch {
			error = 'Import failed.';
		} finally {
			loading = false;
		}
	}
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Import scan results</h1>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<div class="upload-form">
		<label>
			Source
			<select bind:value={source} onchange={() => { preview = null; result = null; }}>
				<option value="nmap">Nmap XML</option>
				<option value="nessus">Nessus (.nessus)</option>
				<option value="openvas">OpenVAS (XML)</option>
				<option value="bloodhound">BloodHound (computers.json)</option>
			</select>
		</label>
		<input type="file" bind:this={fileInput} onchange={handleFileChange} />
		<button onclick={handlePreview} disabled={!selectedFile || loading}>Preview</button>
	</div>

	{#if loading}
		<p>Working…</p>
	{/if}

	{#if preview}
		<section>
			<h2>Preview</h2>
			<p class="muted">
				{preview.finding_count} finding(s), {preview.trust_relationship_count} trust relationship(s)
				will also be imported.
			</p>
			<table>
				<thead>
					<tr>
						<th>Label</th>
						<th>Hostname</th>
						<th>OS</th>
						<th>Addresses</th>
						<th>Services</th>
						<th>Action</th>
					</tr>
				</thead>
				<tbody>
					{#each preview.hosts as host}
						<tr>
							<td>{host.label}</td>
							<td>{host.hostname ?? ''}</td>
							<td>{host.os ?? ''}</td>
							<td>{host.addresses.join(', ')}</td>
							<td>{host.service_count}</td>
							<td>
								<span class="action-badge action-{host.action}">{host.action}</span>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
			<button onclick={handleCommit} disabled={loading}>Confirm import</button>
		</section>
	{/if}

	{#if result}
		<section>
			<h2>Import complete</h2>
			<ul>
				<li>{result.created_hosts} host(s) created</li>
				<li>{result.merged_hosts} host(s) merged into existing hosts</li>
				<li>{result.services_added} service(s) added</li>
				<li>{result.findings_added} finding(s) added</li>
				<li>{result.trust_relationships_added} trust relationship(s) added</li>
			</ul>
			<p><a href={`/engagements/${engagementId}/hosts`}>View hosts &rarr;</a></p>
		</section>
	{/if}
</main>

<style>
	.error {
		color: var(--error);
	}
	.muted {
		color: var(--text-muted);
	}
	.upload-form {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
		margin-bottom: 1.5rem;
		flex-wrap: wrap;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
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
	.action-badge {
		font-size: 0.75rem;
		text-transform: uppercase;
		border-radius: 999px;
		padding: 0.1rem 0.5rem;
	}
	.action-create {
		background: var(--success-bg);
		color: var(--success);
	}
	.action-merge {
		background: var(--warning-bg);
		color: var(--warning);
	}
</style>
