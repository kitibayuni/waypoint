<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { listCredentials, createCredential, deleteCredential, revealCredential } from '$lib/api/credentials';
	import type { Credential } from '$lib/api/credentials';
	import { listUsage, createUsage } from '$lib/api/credential_usage';
	import type { CredentialUsage } from '$lib/api/credential_usage';
	import { listHosts } from '$lib/api/hosts';
	import type { Host } from '$lib/api/hosts';

	const engagementId = $page.params.id as string;

	let credentials = $state<Credential[]>([]);
	let hosts = $state<Host[]>([]);
	let usageByCredential = $state<Record<string, CredentialUsage[]>>({});
	let loading = $state(true);
	let error = $state('');
	let expandedId = $state<string | null>(null);
	let revealedSecret = $state<Record<string, string>>({});

	let newUsername = $state('');
	let newDomain = $state('');
	let newSecret = $state('');
	let newSecretType = $state('plaintext');
	let newSourceHostId = $state('');
	let newOrigin = $state('captured');
	let newNotes = $state('');

	let testHostId = $state('');
	let testResult = $state('works');
	let testPrivilege = $state('');

	const groupedCredentials = $derived.by(() => {
		const byHost = new Map<string, Credential[]>();
		const unassigned: Credential[] = [];
		for (const c of credentials) {
			if (c.source_host_id) {
				const list = byHost.get(c.source_host_id) ?? [];
				list.push(c);
				byHost.set(c.source_host_id, list);
			} else {
				unassigned.push(c);
			}
		}
		const groups: { label: string; credentials: Credential[] }[] = [];
		for (const host of hosts) {
			const list = byHost.get(host.id);
			if (list?.length) groups.push({ label: host.label, credentials: list });
		}
		if (unassigned.length > 0) groups.push({ label: 'Unassigned', credentials: unassigned });
		return groups;
	});

	async function load() {
		loading = true;
		error = '';
		try {
			const [creds, hostList] = await Promise.all([
				listCredentials(engagementId),
				listHosts(engagementId)
			]);
			credentials = creds;
			hosts = hostList;
		} catch {
			error = 'Failed to load credentials.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function handleCreate(e: SubmitEvent) {
		e.preventDefault();
		if (!newUsername.trim() || !newSecret.trim()) return;
		try {
			const cred = await createCredential(engagementId, {
				username: newUsername,
				domain: newDomain || null,
				secret: newSecret,
				secret_type: newSecretType,
				source_host_id: newSourceHostId || null,
				origin: newOrigin,
				notes_md: newNotes
			});
			credentials = [...credentials, cred];
			newUsername = '';
			newDomain = '';
			newSecret = '';
			newSourceHostId = '';
			newNotes = '';
			error = '';
		} catch {
			error = 'Failed to create credential.';
		}
	}

	async function handleDelete(id: string) {
		try {
			await deleteCredential(id);
			credentials = credentials.filter((c) => c.id !== id);
			if (expandedId === id) expandedId = null;
		} catch {
			error = 'Failed to remove credential.';
		}
	}

	async function toggleExpand(credential: Credential) {
		if (expandedId === credential.id) {
			expandedId = null;
			return;
		}
		expandedId = credential.id;
		testHostId = '';
		testResult = 'works';
		testPrivilege = '';
		if (!usageByCredential[credential.id]) {
			try {
				usageByCredential = {
					...usageByCredential,
					[credential.id]: await listUsage(credential.id)
				};
			} catch {
				error = 'Failed to load usage history.';
			}
		}
	}

	async function handleReveal(id: string) {
		try {
			const { secret } = await revealCredential(id);
			revealedSecret = { ...revealedSecret, [id]: secret };
		} catch {
			error = 'Failed to reveal secret.';
		}
	}

	async function handleRecordUsage(credentialId: string) {
		if (!testHostId) return;
		try {
			const usage = await createUsage(credentialId, {
				host_id: testHostId,
				result: testResult,
				privilege: testPrivilege || null
			});
			usageByCredential = {
				...usageByCredential,
				[credentialId]: [...(usageByCredential[credentialId] ?? []), usage]
			};
			testHostId = '';
			testPrivilege = '';
			error = '';
		} catch {
			error = 'Failed to record usage.';
		}
	}

	function untestedHosts(credentialId: string): Host[] {
		const tried = new Set((usageByCredential[credentialId] ?? []).map((u) => u.host_id));
		return hosts.filter((h) => !tried.has(h.id));
	}

	// Falls back to a domain-shaped hint derived from the source host's hostname
	// (e.g. dc01.acme.local -> acme.local) when no domain was typed explicitly --
	// credentials are already grouped by host, so a separate mandatory domain
	// field is often redundant with what the host record already implies.
	function hostDomainHint(sourceHostId: string | null): string {
		if (!sourceHostId) return '';
		const host = hosts.find((h) => h.id === sourceHostId);
		const dot = host?.hostname?.indexOf('.') ?? -1;
		return dot > -1 ? host!.hostname!.slice(dot + 1) : '';
	}
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Credentials</h1>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	<form onsubmit={handleCreate}>
		<h2>New credential</h2>
		<div class="grid">
			<label>
				Username
				<input bind:value={newUsername} required />
			</label>
			<label>
				Domain (optional — inferred from source host if left blank)
				<input bind:value={newDomain} placeholder="e.g. ACME.LOCAL" />
			</label>
			<label>
				Secret
				<input type="password" bind:value={newSecret} required />
			</label>
			<label>
				Secret type
				<select bind:value={newSecretType}>
					<option value="plaintext">plaintext</option>
					<option value="ntlm">ntlm</option>
					<option value="kerb">kerb</option>
					<option value="ssh_key">ssh_key</option>
					<option value="hash_other">hash_other</option>
				</select>
			</label>
			<label>
				Origin
				<select bind:value={newOrigin}>
					<option value="captured">captured</option>
					<option value="cracked">cracked</option>
					<option value="sprayed">sprayed</option>
					<option value="default">default</option>
					<option value="created">created</option>
				</select>
			</label>
			<label>
				Source host
				<select bind:value={newSourceHostId}>
					<option value="">(none)</option>
					{#each hosts as host (host.id)}
						<option value={host.id}>{host.label}</option>
					{/each}
				</select>
			</label>
		</div>
		<label>
			Notes
			<input bind:value={newNotes} placeholder="e.g. found in LSASS dump on DC01" />
		</label>
		<button type="submit">Add credential</button>
	</form>

	{#if loading}
		<p>Loading…</p>
	{:else if credentials.length === 0}
		<p>No credentials yet.</p>
	{:else}
		{#each groupedCredentials as group (group.label)}
			<h2>{group.label}</h2>
			<table>
				<thead>
					<tr>
						<th>Username</th>
						<th>Domain</th>
						<th>Type</th>
						<th>Origin</th>
						<th>Validated</th>
						<th>Notes</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each group.credentials as credential (credential.id)}
						<tr class="clickable" onclick={() => toggleExpand(credential)}>
						<td>{credential.username}</td>
						<td>
							{#if credential.domain}
								{credential.domain}
							{:else}
								<span class="muted">{hostDomainHint(credential.source_host_id)}</span>
							{/if}
						</td>
						<td>{credential.secret_type}</td>
						<td>{credential.origin}</td>
						<td>{credential.validated ? 'yes' : 'no'}</td>
						<td>{credential.notes_md}</td>
						<td>
							<button
								onclick={(e) => {
									e.stopPropagation();
									handleDelete(credential.id);
								}}
							>
								Remove
							</button>
						</td>
					</tr>
					{#if expandedId === credential.id}
						<tr>
							<td colspan="7">
								<div class="expanded">
									<div class="secret-row">
										<button onclick={() => handleReveal(credential.id)}>Reveal secret</button>
										{#if revealedSecret[credential.id]}
											<code>{revealedSecret[credential.id]}</code>
										{/if}
									</div>

									<h3>Usage</h3>
									{#if usageByCredential[credential.id]?.length}
										<ul class="usage-list">
											{#each usageByCredential[credential.id] as usage (usage.id)}
												<li>
													<strong>{usage.host_label}</strong>: {usage.result}
													{#if usage.privilege}({usage.privilege}){/if}
												</li>
											{/each}
										</ul>
									{:else}
										<p class="muted">Not tried against any host yet.</p>
									{/if}

									<div class="inline-form">
										<select bind:value={testHostId}>
											<option value="" disabled selected>Select a host…</option>
											{#each hosts as host (host.id)}
												<option value={host.id}>{host.label}</option>
											{/each}
										</select>
										<select bind:value={testResult}>
											<option value="works">works</option>
											<option value="fails">fails</option>
											<option value="untested">untested</option>
										</select>
										<select bind:value={testPrivilege}>
											<option value="">(no privilege)</option>
											<option value="user">user</option>
											<option value="admin">admin</option>
											<option value="domain_admin">domain_admin</option>
											<option value="system">system</option>
										</select>
										<button onclick={() => handleRecordUsage(credential.id)}>Record test</button>
									</div>

									{#if usageByCredential[credential.id] && untestedHosts(credential.id).length > 0}
										<p class="reuse-flag">
											&#9888; Untested against: {untestedHosts(credential.id)
												.map((h) => h.label)
												.join(', ')}
										</p>
									{/if}
								</div>
							</td>
						</tr>
					{/if}
				{/each}
			</tbody>
		</table>
		{/each}
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
	table {
		border-collapse: collapse;
		width: 100%;
	}
	th,
	td {
		text-align: left;
		padding: 0.4rem 0.6rem;
		border-bottom: 1px solid var(--border);
	}
	tr.clickable {
		cursor: pointer;
	}
	tr.clickable:hover {
		background: var(--surface-2);
	}
	.expanded {
		padding: 0.75rem;
		background: var(--surface);
		border-radius: 6px;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.secret-row {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}
	.usage-list {
		margin: 0;
		padding-left: 1.2rem;
	}
	.muted {
		color: var(--text-muted);
		font-size: 0.9rem;
	}
	.inline-form {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		align-items: center;
	}
	.reuse-flag {
		color: var(--warning);
		background: var(--warning-bg);
		border: 1px solid var(--warning);
		border-radius: 4px;
		padding: 0.4rem 0.6rem;
		font-size: 0.85rem;
	}
</style>
