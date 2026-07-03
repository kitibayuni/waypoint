<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { getEngagement, updateEngagement } from '$lib/api/engagements';
	import type { Engagement } from '$lib/api/engagements';
	import { listScope, createScopeItem, deleteScopeItem } from '$lib/api/scope';
	import type { ScopeItem } from '$lib/api/scope';
	import { listMembers, addMember, updateMemberRole, removeMember } from '$lib/api/members';
	import type { Member } from '$lib/api/members';
	import Dashboard from '$lib/components/Dashboard.svelte';

	const engagementId = $page.params.id as string;

	let engagement = $state<Engagement | null>(null);
	let scopeItems = $state<ScopeItem[]>([]);
	let members = $state<Member[]>([]);
	let notesDraft = $state('');
	let loading = $state(true);
	let error = $state('');

	let newScopeKind = $state('ip');
	let newScopeValue = $state('');

	let newMemberEmail = $state('');
	let newMemberRole = $state('tester');

	async function load() {
		loading = true;
		error = '';
		try {
			const [e, scope, mem] = await Promise.all([
				getEngagement(engagementId),
				listScope(engagementId),
				listMembers(engagementId)
			]);
			engagement = e;
			scopeItems = scope;
			members = mem;
			notesDraft = e.global_notes_md;
		} catch {
			error = 'Failed to load engagement.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function saveNotes() {
		if (!engagement) return;
		try {
			engagement = await updateEngagement(engagementId, {
				name: engagement.name,
				status: engagement.status,
				start_date: engagement.start_date,
				end_date: engagement.end_date,
				global_notes_md: notesDraft
			});
			error = '';
		} catch {
			error = 'Failed to save notes.';
		}
	}

	async function handleAddScope(e: SubmitEvent) {
		e.preventDefault();
		if (!newScopeValue.trim()) return;
		try {
			const item = await createScopeItem(engagementId, { kind: newScopeKind, value: newScopeValue });
			scopeItems = [...scopeItems, item];
			newScopeValue = '';
		} catch {
			error = 'Failed to add scope item.';
		}
	}

	async function handleDeleteScope(id: string) {
		try {
			await deleteScopeItem(engagementId, id);
			scopeItems = scopeItems.filter((s) => s.id !== id);
		} catch {
			error = 'Failed to remove scope item.';
		}
	}

	async function handleAddMember(e: SubmitEvent) {
		e.preventDefault();
		if (!newMemberEmail.trim()) return;
		try {
			const member = await addMember(engagementId, newMemberEmail, newMemberRole);
			members = [...members, member];
			newMemberEmail = '';
		} catch {
			error = 'Failed to add member (check the email and your permissions).';
		}
	}

	async function handleRoleChange(userId: string, role: string) {
		try {
			const updated = await updateMemberRole(engagementId, userId, role);
			members = members.map((m) => (m.user_id === userId ? updated : m));
		} catch {
			error = 'Failed to update role.';
		}
	}

	async function handleRemoveMember(userId: string) {
		try {
			await removeMember(engagementId, userId);
			members = members.filter((m) => m.user_id !== userId);
		} catch {
			error = 'Failed to remove member.';
		}
	}
</script>

<main>
	{#if loading}
		<p>Loading…</p>
	{:else if !engagement}
		<p class="error">{error || 'Engagement not found.'}</p>
	{:else}
		<p><a href="/engagements">&larr; All engagements</a></p>
		<h1>{engagement.name}</h1>
		<p class="meta">
			Client: {engagement.client_name} · Status: {engagement.status}
			{#if engagement.start_date || engagement.end_date}
				· {engagement.start_date ?? '?'} &rarr; {engagement.end_date ?? '?'}
			{/if}
		</p>
		<p><a href={`/engagements/${engagementId}/hosts`}>View hosts &rarr;</a></p>
		<p><a href={`/engagements/${engagementId}/credentials`}>View credentials &rarr;</a></p>
		<p><a href={`/engagements/${engagementId}/graph`}>View attack graph &rarr;</a></p>
		<p><a href={`/engagements/${engagementId}/templates`}>View templates &rarr;</a></p>
		<p><a href={`/engagements/${engagementId}/findings`}>View findings &rarr;</a></p>
		<p><a href={`/engagements/${engagementId}/search`}>Search &rarr;</a></p>

		{#if error}
			<p class="error">{error}</p>
		{/if}

		<Dashboard {engagementId} />

		<section>
			<h2>Global notes</h2>
			<textarea bind:value={notesDraft} rows="8"></textarea>
			<button onclick={saveNotes}>Save notes</button>
		</section>

		<section>
			<h2>Scope</h2>
			<table>
				<thead>
					<tr>
						<th>Kind</th>
						<th>Value</th>
						<th>In scope</th>
						<th>Note</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each scopeItems as item (item.id)}
						<tr>
							<td>{item.kind}</td>
							<td>{item.value}</td>
							<td>{item.in_scope ? 'yes' : 'no'}</td>
							<td>{item.note ?? ''}</td>
							<td><button onclick={() => handleDeleteScope(item.id)}>Remove</button></td>
						</tr>
					{/each}
				</tbody>
			</table>
			<form onsubmit={handleAddScope}>
				<select bind:value={newScopeKind}>
					<option value="ip">IP</option>
					<option value="cidr">CIDR</option>
					<option value="domain">Domain</option>
					<option value="url">URL</option>
					<option value="asn">ASN</option>
					<option value="exclusion">Exclusion</option>
				</select>
				<input bind:value={newScopeValue} placeholder="e.g. 10.10.10.0/24" required />
				<button type="submit">Add scope item</button>
			</form>
		</section>

		<section>
			<h2>Team members</h2>
			<table>
				<thead>
					<tr>
						<th>Name</th>
						<th>Email</th>
						<th>Role</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each members as member (member.user_id)}
						<tr>
							<td>{member.display_name}</td>
							<td>{member.email}</td>
							<td>
								<select
									value={member.role}
									onchange={(e) =>
										handleRoleChange(member.user_id, (e.target as HTMLSelectElement).value)}
								>
									<option value="viewer">viewer</option>
									<option value="tester">tester</option>
									<option value="lead">lead</option>
								</select>
							</td>
							<td><button onclick={() => handleRemoveMember(member.user_id)}>Remove</button></td>
						</tr>
					{/each}
				</tbody>
			</table>
			<form onsubmit={handleAddMember}>
				<input type="email" bind:value={newMemberEmail} placeholder="user@example.com" required />
				<select bind:value={newMemberRole}>
					<option value="viewer">viewer</option>
					<option value="tester">tester</option>
					<option value="lead">lead</option>
				</select>
				<button type="submit">Add member</button>
			</form>
		</section>
	{/if}
</main>

<style>
	.error {
		color: #c0392b;
	}
	.meta {
		color: #555;
		margin-bottom: 1rem;
	}
	section {
		margin-top: 2rem;
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
	form {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}
	textarea {
		width: 100%;
		font-family: inherit;
	}
</style>
