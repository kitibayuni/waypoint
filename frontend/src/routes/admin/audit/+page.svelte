<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { currentUser } from '$lib/stores/auth';
	import { listAuditLog } from '$lib/api/audit';
	import type { AuditEntry } from '$lib/api/audit';

	let entries = $state<AuditEntry[]>([]);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		if (!$currentUser?.is_admin) {
			goto('/');
			return;
		}
		try {
			entries = await listAuditLog();
		} catch {
			error = 'Failed to load audit log.';
		} finally {
			loading = false;
		}
	});
</script>

<main>
	<p><a href="/">&larr; Home</a></p>
	<h1>Audit log</h1>

	{#if loading}
		<p>Loading…</p>
	{:else if error}
		<p class="error">{error}</p>
	{:else if entries.length === 0}
		<p>No audit entries yet.</p>
	{:else}
		<ul class="audit-list">
			{#each entries as entry (entry.id)}
				<li>
					<div class="audit-head">
						<strong>{entry.action}</strong>
						<span class="subject">{entry.subject_type}</span>
						<code>{entry.subject_id}</code>
						<span>{entry.actor_email ?? 'unknown'}</span>
						<time>{new Date(entry.at).toLocaleString()}</time>
					</div>
					<div class="audit-diff">
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
</main>

<style>
	.error {
		color: #c0392b;
	}
	.audit-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}
	.audit-list > li {
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.6rem 0.75rem;
	}
	.audit-head {
		display: flex;
		gap: 0.75rem;
		align-items: baseline;
		flex-wrap: wrap;
		font-size: 0.85rem;
		color: #555;
		margin-bottom: 0.4rem;
	}
	.audit-head strong {
		text-transform: uppercase;
		color: #333;
	}
	.subject {
		font-weight: 600;
	}
	.audit-diff {
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
