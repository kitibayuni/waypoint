<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { listEngagements, createEngagement } from '$lib/api/engagements';
	import { listClients, createClient } from '$lib/api/clients';
	import type { Engagement } from '$lib/api/engagements';
	import type { Client } from '$lib/api/clients';

	let engagements = $state<Engagement[]>([]);
	let clients = $state<Client[]>([]);
	let loading = $state(true);
	let error = $state('');

	let newEngagementName = $state('');
	let newEngagementClientId = $state('');
	let newClientName = $state('');
	let showNewClient = $state(false);

	async function load() {
		loading = true;
		try {
			[engagements, clients] = await Promise.all([listEngagements(), listClients()]);
		} catch {
			error = 'Failed to load engagements.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function handleCreateClient() {
		if (!newClientName.trim()) return;
		const client = await createClient({ name: newClientName });
		clients = [...clients, client];
		newEngagementClientId = client.id;
		newClientName = '';
		showNewClient = false;
	}

	async function handleCreateEngagement(e: SubmitEvent) {
		e.preventDefault();
		if (!newEngagementName.trim() || !newEngagementClientId) return;
		try {
			const engagement = await createEngagement({
				client_id: newEngagementClientId,
				name: newEngagementName
			});
			goto(`/engagements/${engagement.id}`);
		} catch {
			error = 'Failed to create engagement.';
		}
	}
</script>

<main>
	<h1>Engagements</h1>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<form onsubmit={handleCreateEngagement}>
		<h2>New engagement</h2>
		<label>
			Name
			<input bind:value={newEngagementName} required />
		</label>
		<label>
			Client
			<select bind:value={newEngagementClientId} required>
				<option value="" disabled selected>Select a client</option>
				{#each clients as client (client.id)}
					<option value={client.id}>{client.name}</option>
				{/each}
			</select>
		</label>
		<button type="button" onclick={() => (showNewClient = !showNewClient)}>
			{showNewClient ? 'Cancel' : '+ New client'}
		</button>
		{#if showNewClient}
			<div class="new-client">
				<input bind:value={newClientName} placeholder="Client name" />
				<button type="button" onclick={handleCreateClient}>Add client</button>
			</div>
		{/if}
		<button type="submit">Create engagement</button>
	</form>

	{#if loading}
		<p>Loading…</p>
	{:else}
		<ul>
			{#each engagements as engagement (engagement.id)}
				<li>
					<a href={`/engagements/${engagement.id}`}>{engagement.name}</a>
					— {engagement.client_name} ({engagement.status})
				</li>
			{/each}
		</ul>
	{/if}
</main>

<style>
	.error {
		color: var(--error);
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-width: 24rem;
		margin-bottom: 1.5rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
	}
	.new-client {
		display: flex;
		gap: 0.5rem;
	}
</style>
