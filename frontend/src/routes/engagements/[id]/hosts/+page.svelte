<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { listHosts, createHost } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';
	import HostCard from '$lib/components/HostCard.svelte';

	const engagementId = $page.params.id as string;

	let hosts = $state<Host[]>([]);
	let loading = $state(true);
	let error = $state('');

	let newLabel = $state('');
	let newHostname = $state('');
	let newOs = $state('');
	let newAddresses = $state('');
	let newTags = $state('');

	async function load() {
		loading = true;
		try {
			hosts = await listHosts(engagementId);
		} catch {
			error = 'Failed to load hosts.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function handleCreate(e: SubmitEvent) {
		e.preventDefault();
		if (!newLabel.trim()) return;
		try {
			const host = await createHost(engagementId, {
				label: newLabel,
				hostname: newHostname || null,
				os: newOs || null,
				addresses: newAddresses
					.split(',')
					.map((s) => s.trim())
					.filter(Boolean),
				tags: newTags
					.split(',')
					.map((s) => s.trim())
					.filter(Boolean)
			});
			hosts = [...hosts, host];
			newLabel = '';
			newHostname = '';
			newOs = '';
			newAddresses = '';
			newTags = '';
			error = '';
		} catch {
			error = 'Failed to create host (check the IP address format).';
		}
	}
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Hosts</h1>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<form onsubmit={handleCreate}>
		<h2>New host</h2>
		<label>
			Label
			<input bind:value={newLabel} required placeholder="e.g. DC01" />
		</label>
		<label>
			Hostname
			<input bind:value={newHostname} placeholder="e.g. dc01.corp.local" />
		</label>
		<label>
			OS
			<input bind:value={newOs} placeholder="e.g. Windows Server 2019" />
		</label>
		<label>
			IP addresses (comma-separated)
			<input bind:value={newAddresses} placeholder="10.10.10.5, 10.10.10.6" />
		</label>
		<label>
			Tags (comma-separated)
			<input bind:value={newTags} placeholder="domain-controller, critical" />
		</label>
		<button type="submit">Add host</button>
	</form>

	{#if loading}
		<p>Loading…</p>
	{:else if hosts.length === 0}
		<p>No hosts yet.</p>
	{:else}
		<div class="host-list">
			{#each hosts as host (host.id)}
				<HostCard {host} />
			{/each}
		</div>
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
	.host-list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
		gap: 0.75rem;
	}
</style>
