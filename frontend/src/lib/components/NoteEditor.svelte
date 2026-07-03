<script lang="ts">
	import MarkdownIt from 'markdown-it';
	import { createNote, updateNote, deleteNote } from '$lib/api/notes';
	import type { Note } from '$lib/api/notes';

	let {
		engagementId,
		subjectType,
		subjectId,
		notes = $bindable()
	}: {
		engagementId: string;
		subjectType: string;
		subjectId: string;
		notes: Note[];
	} = $props();

	// Default options: html is false, so raw HTML in note content is
	// escaped rather than rendered -- required to keep {@html} below safe.
	const md = new MarkdownIt();

	let creating = $state(false);
	let newTitle = $state('');
	let newBody = $state('');
	let editingId = $state<string | null>(null);
	let editTitle = $state('');
	let editBody = $state('');
	let previewIds = $state<Set<string>>(new Set());
	let error = $state('');

	async function handleCreate() {
		if (!newBody.trim()) return;
		try {
			const note = await createNote({
				engagement_id: engagementId,
				subject_type: subjectType,
				subject_id: subjectId,
				title: newTitle || null,
				body_md: newBody
			});
			notes = [...notes, note];
			newTitle = '';
			newBody = '';
			creating = false;
			error = '';
		} catch {
			error = 'Failed to create note.';
		}
	}

	function startEdit(note: Note) {
		editingId = note.id;
		editTitle = note.title ?? '';
		editBody = note.body_md;
	}

	async function saveEdit(id: string) {
		try {
			const updated = await updateNote(id, { title: editTitle || null, body_md: editBody });
			notes = notes.map((n) => (n.id === id ? updated : n));
			editingId = null;
			error = '';
		} catch {
			error = 'Failed to save note.';
		}
	}

	async function handleDelete(id: string) {
		try {
			await deleteNote(id);
			notes = notes.filter((n) => n.id !== id);
		} catch {
			error = 'Failed to remove note.';
		}
	}

	function togglePreview(id: string) {
		const next = new Set(previewIds);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		previewIds = next;
	}
</script>

<div class="notes">
	{#if error}<p class="error">{error}</p>{/if}

	{#each notes as note (note.id)}
		<article class="note">
			{#if editingId === note.id}
				<input bind:value={editTitle} placeholder="Title (optional)" />
				<textarea bind:value={editBody} rows="6"></textarea>
				<div class="actions">
					<button onclick={() => saveEdit(note.id)}>Save</button>
					<button onclick={() => (editingId = null)}>Cancel</button>
				</div>
			{:else}
				<div class="note-header">
					{#if note.title}<h3>{note.title}</h3>{/if}
					<div class="actions">
						<button onclick={() => togglePreview(note.id)}>
							{previewIds.has(note.id) ? 'Raw' : 'Preview'}
						</button>
						<button onclick={() => startEdit(note)}>Edit</button>
						<button onclick={() => handleDelete(note.id)}>Delete</button>
					</div>
				</div>
				{#if previewIds.has(note.id)}
					<div class="rendered">{@html md.render(note.body_md)}</div>
				{:else}
					<pre>{note.body_md}</pre>
				{/if}
			{/if}
		</article>
	{/each}

	{#if creating}
		<div class="note new-note">
			<input bind:value={newTitle} placeholder="Title (optional)" />
			<textarea bind:value={newBody} rows="6" placeholder="Markdown…"></textarea>
			<div class="actions">
				<button onclick={handleCreate}>Save note</button>
				<button onclick={() => (creating = false)}>Cancel</button>
			</div>
		</div>
	{:else}
		<button onclick={() => (creating = true)}>+ Add note</button>
	{/if}
</div>

<style>
	.error {
		color: var(--error);
	}
	.notes {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.note {
		border: 1px solid var(--border);
		border-radius: 6px;
		padding: 0.75rem;
	}
	.note-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}
	.note h3 {
		margin: 0;
	}
	.note pre {
		white-space: pre-wrap;
		font-family: inherit;
		margin: 0.5rem 0 0;
	}
	.rendered {
		margin-top: 0.5rem;
	}
	.actions {
		display: flex;
		gap: 0.4rem;
	}
	.new-note {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	textarea {
		font-family: inherit;
		width: 100%;
	}
</style>
