<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { listTemplates, createTemplate, deleteTemplate, instantiateTemplate } from '$lib/api/templates';
	import type { Template } from '$lib/api/templates';
	import { listHosts } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';

	const engagementId = $page.params.id as string;

	let templates = $state<Template[]>([]);
	let hosts = $state<Host[]>([]);
	let loading = $state(true);
	let error = $state('');
	let message = $state('');

	let newKind = $state('host');
	let newName = $state('');
	let newDescription = $state('');
	let newBodyJson = $state('{}');

	let hostLabelDraft = $state<Record<string, string>>({});
	let checklistHostDraft = $state<Record<string, string>>({});
	let noteHostDraft = $state<Record<string, string>>({});

	async function load() {
		loading = true;
		error = '';
		try {
			const [tpls, hostList] = await Promise.all([listTemplates(), listHosts(engagementId)]);
			templates = tpls;
			hosts = hostList;
		} catch {
			error = 'Failed to load templates.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function handleCreate(e: SubmitEvent) {
		e.preventDefault();
		if (!newName.trim()) return;
		let body: unknown;
		try {
			body = JSON.parse(newBodyJson || '{}');
		} catch {
			error = 'Template body must be valid JSON.';
			return;
		}
		try {
			const tpl = await createTemplate({
				kind: newKind,
				name: newName,
				description: newDescription || null,
				body
			});
			templates = [...templates, tpl];
			newName = '';
			newDescription = '';
			newBodyJson = '{}';
			error = '';
		} catch {
			error = 'Failed to create template.';
		}
	}

	async function handleDelete(id: string) {
		try {
			await deleteTemplate(id);
			templates = templates.filter((t) => t.id !== id);
		} catch {
			error = 'Failed to remove template.';
		}
	}

	async function handleUseHostTemplate(template: Template) {
		try {
			const result = await instantiateTemplate(template.id, {
				engagement_id: engagementId,
				name: hostLabelDraft[template.id] || undefined
			});
			goto(`/engagements/${engagementId}/hosts/${result.id}`);
		} catch {
			error = 'Failed to create host from template.';
		}
	}

	async function handleUseChecklistTemplate(template: Template) {
		const hostId = checklistHostDraft[template.id];
		if (!hostId) {
			error = 'Select a host first.';
			return;
		}
		try {
			await instantiateTemplate(template.id, { host_id: hostId });
			error = '';
			goto(`/engagements/${engagementId}/hosts/${hostId}`);
		} catch {
			error = 'Failed to attach checklist.';
		}
	}

	async function handleUseFindingTemplate(template: Template) {
		try {
			await instantiateTemplate(template.id, { engagement_id: engagementId });
			message = `Finding "${template.name}" created for this engagement.`;
			error = '';
		} catch {
			error = 'Failed to create finding.';
		}
	}

	async function handleUseNoteTemplate(template: Template) {
		const hostId = noteHostDraft[template.id];
		if (!hostId) {
			error = 'Select a host first.';
			return;
		}
		try {
			await instantiateTemplate(template.id, {
				engagement_id: engagementId,
				subject_type: 'host',
				subject_id: hostId
			});
			error = '';
			goto(`/engagements/${engagementId}/hosts/${hostId}`);
		} catch {
			error = 'Failed to attach note.';
		}
	}

	function templatesByKind(kind: string): Template[] {
		return templates.filter((t) => t.kind === kind);
	}
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Templates</h1>

	{#if error}
		<p class="error">{error}</p>
	{/if}
	{#if message}
		<p class="message">{message}</p>
	{/if}

	<form onsubmit={handleCreate}>
		<h2>New template</h2>
		<div class="grid">
			<label>
				Kind
				<select bind:value={newKind}>
					<option value="host">host</option>
					<option value="checklist">checklist</option>
					<option value="finding">finding</option>
					<option value="note">note</option>
					<option value="engagement">engagement</option>
				</select>
			</label>
			<label>
				Name
				<input bind:value={newName} required />
			</label>
		</div>
		<label>
			Description
			<input bind:value={newDescription} />
		</label>
		<label>
			Body (JSON)
			<textarea bind:value={newBodyJson} rows="6"></textarea>
		</label>
		<button type="submit">Create template</button>
	</form>

	{#if loading}
		<p>Loading…</p>
	{:else}
		<h2>Host templates</h2>
		{#each templatesByKind('host') as template (template.id)}
			<div class="template-card">
				<div class="template-header">
					<strong>{template.name}</strong>
					<button onclick={() => handleDelete(template.id)}>Delete</button>
				</div>
				{#if template.description}<p class="muted">{template.description}</p>{/if}
				<div class="inline-form">
					<input
						bind:value={hostLabelDraft[template.id]}
						placeholder={`Label (default: ${template.name})`}
					/>
					<button onclick={() => handleUseHostTemplate(template)}>New host from template</button>
				</div>
			</div>
		{/each}

		<h2>Checklist templates</h2>
		{#each templatesByKind('checklist') as template (template.id)}
			<div class="template-card">
				<div class="template-header">
					<strong>{template.name}</strong>
					<button onclick={() => handleDelete(template.id)}>Delete</button>
				</div>
				{#if template.description}<p class="muted">{template.description}</p>{/if}
				<div class="inline-form">
					<select bind:value={checklistHostDraft[template.id]}>
						<option value="" disabled selected>Select a host…</option>
						{#each hosts as host (host.id)}
							<option value={host.id}>{host.label}</option>
						{/each}
					</select>
					<button onclick={() => handleUseChecklistTemplate(template)}>Attach to host</button>
				</div>
			</div>
		{/each}

		<h2>Finding templates</h2>
		{#each templatesByKind('finding') as template (template.id)}
			<div class="template-card">
				<div class="template-header">
					<strong>{template.name}</strong>
					<button onclick={() => handleDelete(template.id)}>Delete</button>
				</div>
				{#if template.description}<p class="muted">{template.description}</p>{/if}
				<button onclick={() => handleUseFindingTemplate(template)}>
					Create finding in this engagement
				</button>
			</div>
		{/each}

		<h2>Note templates</h2>
		{#each templatesByKind('note') as template (template.id)}
			<div class="template-card">
				<div class="template-header">
					<strong>{template.name}</strong>
					<button onclick={() => handleDelete(template.id)}>Delete</button>
				</div>
				{#if template.description}<p class="muted">{template.description}</p>{/if}
				<div class="inline-form">
					<select bind:value={noteHostDraft[template.id]}>
						<option value="" disabled selected>Select a host…</option>
						{#each hosts as host (host.id)}
							<option value={host.id}>{host.label}</option>
						{/each}
					</select>
					<button onclick={() => handleUseNoteTemplate(template)}>Attach to host</button>
				</div>
			</div>
		{/each}

		<h2>Engagement templates</h2>
		<p class="muted">
			Used when starting a brand new engagement from the main engagements list, not from within
			this one.
		</p>
		{#each templatesByKind('engagement') as template (template.id)}
			<div class="template-card">
				<div class="template-header">
					<strong>{template.name}</strong>
					<button onclick={() => handleDelete(template.id)}>Delete</button>
				</div>
				{#if template.description}<p class="muted">{template.description}</p>{/if}
			</div>
		{/each}
	{/if}
</main>

<style>
	.error {
		color: #c0392b;
	}
	.message {
		color: #2a7a2a;
	}
	.muted {
		color: #777;
		font-size: 0.9rem;
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-width: 32rem;
		margin-bottom: 1.5rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
		gap: 0.5rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
	}
	textarea {
		font-family: monospace;
		font-size: 0.85rem;
	}
	.template-card {
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.75rem;
		margin-bottom: 0.75rem;
	}
	.template-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}
	.inline-form {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
		margin-top: 0.5rem;
	}
</style>
