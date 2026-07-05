<script lang="ts">
	import { createHost, deleteHost } from '$lib/api/hosts';
	import { createCredential, deleteCredential } from '$lib/api/credentials';
	import { createFinding } from '$lib/api/findings';
	import { createTrustRelationship } from '$lib/api/trust_relationships';
	import { createService, deleteService, SERVICE_NAMES } from '$lib/api/services';

	let {
		info,
		engagementId,
		onClose,
		onChanged
	}: {
		info: {
			x: number;
			y: number;
			target: 'background' | 'host' | 'credential' | 'service';
			nodeId?: string;
			/** The owning host id, only set when target is 'service'. */
			hostId?: string;
		};
		engagementId: string;
		onClose: () => void;
		onChanged: () => void;
	} = $props();

	let menuEl = $state<HTMLElement>();
	let mode = $state<'menu' | 'add-host' | 'add-credential' | 'add-finding' | 'add-service'>('menu');
	let error = $state('');

	let hostLabel = $state('');
	let hostHostname = $state('');
	let hostAddresses = $state('');
	let hostKind = $state('session');

	let credUsername = $state('');
	let credDomain = $state('');
	let credSecret = $state('');
	let credSecretType = $state('plaintext');
	let credOrigin = $state('captured');
	let credNotes = $state('');

	let findingTitle = $state('');
	let findingSeverity = $state('');

	let svcPort = $state<number | ''>('');
	let svcProtocol = $state('tcp');
	let svcName = $state('');
	let svcDisplayName = $state('');
	let svcVersion = $state('');

	// Re-clamp whenever the menu's content changes size (e.g. switching from the
	// top-level menu to a bigger add-form), not just once at mount.
	$effect(() => {
		void mode;
		if (!menuEl) return;
		const rect = menuEl.getBoundingClientRect();
		const overflowX = rect.right - window.innerWidth;
		const overflowY = rect.bottom - window.innerHeight;
		menuEl.style.left = `${overflowX > 0 ? Math.max(0, info.x - overflowX) : info.x}px`;
		menuEl.style.top = `${overflowY > 0 ? Math.max(0, info.y - overflowY) : info.y}px`;
	});

	function handleDocumentClick(e: MouseEvent) {
		// Use composedPath() rather than e.target: clicking a menu item can swap
		// <ul> for a <form> synchronously (mode change), detaching the clicked
		// button from the DOM before this bubbles to document -- at which point
		// menuEl.contains(e.target) would wrongly read as "outside" and close the
		// whole menu instead of switching to the sub-form. composedPath() is
		// captured at dispatch time, before any of that DOM mutation happens.
		if (menuEl && !e.composedPath().includes(menuEl)) onClose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	async function handleAddHost(e: SubmitEvent) {
		e.preventDefault();
		if (!hostLabel.trim()) return;
		try {
			const created = await createHost(engagementId, {
				label: hostLabel,
				hostname: hostHostname || null,
				addresses: hostAddresses
					.split(',')
					.map((s) => s.trim())
					.filter(Boolean),
				source_service_id: info.target === 'service' ? info.nodeId : null
			});
			// Pivoting from either a host or one of its services also logs a normal
			// host-to-host trust relationship, so the new host stays connected into
			// the regular attack graph -- not just reachable via the service arrow.
			const fromHostId = info.target === 'host' ? info.nodeId : info.hostId;
			if (fromHostId) {
				await createTrustRelationship(engagementId, {
					from_host_id: fromHostId,
					to_host_id: created.id,
					kind: hostKind,
					note: null
				});
			}
			onChanged();
			onClose();
		} catch {
			error = 'Failed to add host (check the IP address format).';
		}
	}

	async function handleAddCredential(e: SubmitEvent) {
		e.preventDefault();
		if (!credUsername.trim() || !credSecret.trim()) return;
		try {
			await createCredential(engagementId, {
				username: credUsername,
				domain: credDomain || null,
				secret: credSecret,
				secret_type: credSecretType,
				origin: credOrigin,
				notes_md: credNotes,
				source_host_id: info.target === 'host' ? info.nodeId : (info.hostId ?? null),
				source_service_id: info.target === 'service' ? info.nodeId : null
			});
			onChanged();
			onClose();
		} catch {
			error = 'Failed to add credential.';
		}
	}

	async function handleAddFinding(e: SubmitEvent) {
		e.preventDefault();
		if (!findingTitle.trim()) return;
		try {
			const affectedHostId = info.target === 'host' ? info.nodeId : info.hostId;
			await createFinding(engagementId, {
				title: findingTitle,
				severity: findingSeverity || null,
				affected_host_ids: affectedHostId ? [affectedHostId] : []
			});
			onChanged();
			onClose();
		} catch {
			error = 'Failed to add finding.';
		}
	}

	async function handleAddService(e: SubmitEvent) {
		e.preventDefault();
		if (svcPort === '' || !info.nodeId) return;
		try {
			await createService(info.nodeId, {
				port: Number(svcPort),
				protocol: svcProtocol,
				name: svcName || null,
				display_name: svcDisplayName || null,
				version: svcVersion || null
			});
			onChanged();
			onClose();
		} catch {
			error = 'Failed to add service.';
		}
	}

	async function handleDelete() {
		if (!info.nodeId) return;
		if (!confirm(`Delete this ${info.target}? This cannot be undone.`)) return;
		try {
			if (info.target === 'host') {
				await deleteHost(info.nodeId);
			} else if (info.target === 'credential') {
				await deleteCredential(info.nodeId);
			} else if (info.target === 'service' && info.hostId) {
				await deleteService(info.hostId, info.nodeId);
			}
			onChanged();
			onClose();
		} catch {
			error = `Failed to delete ${info.target}.`;
		}
	}
</script>

<svelte:document onclick={handleDocumentClick} onkeydown={handleKeydown} />

<div class="menu" style={`left: ${info.x}px; top: ${info.y}px;`} bind:this={menuEl}>
	{#if error}
		<p class="error">{error}</p>
	{/if}

	{#if mode === 'menu'}
		<ul>
			<li><button type="button" onclick={() => (mode = 'add-host')}>+ Add host</button></li>
			<li><button type="button" onclick={() => (mode = 'add-credential')}>+ Add credential</button></li>
			<li><button type="button" onclick={() => (mode = 'add-finding')}>+ Add finding</button></li>
			{#if info.target === 'host'}
				<li><button type="button" onclick={() => (mode = 'add-service')}>+ Add service</button></li>
			{/if}
			{#if info.target !== 'background'}
				<li>
					<button type="button" class="danger" onclick={handleDelete}>
						&minus; Delete {info.target}
					</button>
				</li>
			{/if}
		</ul>
	{:else if mode === 'add-host'}
		<form onsubmit={handleAddHost}>
			<h3>
				Add host{info.target === 'host'
					? ' (pivot from this host)'
					: info.target === 'service'
						? ' (pivot from this service)'
						: ''}
			</h3>
			<label>
				Label
				<input bind:value={hostLabel} required placeholder="e.g. WEB02" />
			</label>
			<label>
				Hostname
				<input bind:value={hostHostname} placeholder="e.g. web02.corp.local" />
			</label>
			<label>
				IP addresses (comma-separated)
				<input bind:value={hostAddresses} placeholder="10.10.10.7" />
			</label>
			{#if info.target === 'host' || info.target === 'service'}
				<label>
					Relationship kind
					<select bind:value={hostKind}>
						<option value="session">session</option>
						<option value="domain_trust">domain trust</option>
						<option value="admin_of">admin of</option>
						<option value="shares_creds">shares creds</option>
					</select>
				</label>
			{/if}
			<div class="actions">
				<button type="submit">Add</button>
				<button type="button" onclick={() => (mode = 'menu')}>Back</button>
			</div>
		</form>
	{:else if mode === 'add-credential'}
		<form onsubmit={handleAddCredential}>
			<h3>Add credential</h3>
			<label>
				Username
				<input bind:value={credUsername} required placeholder="e.g. jsmith" />
			</label>
			<label>
				Domain
				<input bind:value={credDomain} placeholder="e.g. corp.local" />
			</label>
			<label>
				Secret
				<input bind:value={credSecret} required type="text" placeholder="password/hash" />
			</label>
			<label>
				Type
				<select bind:value={credSecretType}>
					<option value="plaintext">plaintext</option>
					<option value="ntlm">ntlm</option>
					<option value="kerb">kerb</option>
					<option value="ssh_key">ssh_key</option>
					<option value="hash_other">hash_other</option>
				</select>
			</label>
			<label>
				Origin
				<select bind:value={credOrigin}>
					<option value="captured">captured</option>
					<option value="cracked">cracked</option>
					<option value="sprayed">sprayed</option>
					<option value="default">default</option>
					<option value="created">created</option>
				</select>
			</label>
			<label>
				Notes
				<input bind:value={credNotes} placeholder="e.g. found in LSASS dump" />
			</label>
			<div class="actions">
				<button type="submit">Add</button>
				<button type="button" onclick={() => (mode = 'menu')}>Back</button>
			</div>
		</form>
	{:else if mode === 'add-finding'}
		<form onsubmit={handleAddFinding}>
			<h3>Add finding</h3>
			<label>
				Title
				<input bind:value={findingTitle} required placeholder="e.g. Weak SMB signing" />
			</label>
			<label>
				Severity
				<select bind:value={findingSeverity}>
					<option value="">(none)</option>
					<option value="low">low</option>
					<option value="medium">medium</option>
					<option value="high">high</option>
					<option value="critical">critical</option>
				</select>
			</label>
			<div class="actions">
				<button type="submit">Add</button>
				<button type="button" onclick={() => (mode = 'menu')}>Back</button>
			</div>
		</form>
	{:else}
		<form onsubmit={handleAddService}>
			<h3>Add service</h3>
			<label>
				Port
				<input type="number" min="0" max="65535" bind:value={svcPort} required placeholder="e.g. 445" />
			</label>
			<label>
				Protocol
				<select bind:value={svcProtocol}>
					<option value="tcp">tcp</option>
					<option value="udp">udp</option>
				</select>
			</label>
			<label>
				Service
				<select bind:value={svcName}>
					<option value="" disabled selected>Service…</option>
					{#each SERVICE_NAMES as svc (svc)}
						<option value={svc}>{svc}</option>
					{/each}
				</select>
			</label>
			<label>
				Display name
				<input bind:value={svcDisplayName} placeholder="display name (optional)" />
			</label>
			<label>
				Version
				<input bind:value={svcVersion} placeholder="version (optional)" />
			</label>
			<div class="actions">
				<button type="submit">Add</button>
				<button type="button" onclick={() => (mode = 'menu')}>Back</button>
			</div>
		</form>
	{/if}
</div>

<style>
	.menu {
		position: fixed;
		z-index: 60;
		min-width: 12rem;
		max-width: 18rem;
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: 6px;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
		padding: 0.4rem;
	}
	.error {
		color: var(--error);
		font-size: 0.8rem;
		margin: 0.2rem 0.3rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}
	ul li button {
		width: 100%;
		text-align: left;
		background: none;
		border: none;
		padding: 0.4rem 0.5rem;
		cursor: pointer;
		font: inherit;
		border-radius: 4px;
	}
	ul li button:hover {
		background: var(--surface-2);
	}
	ul li button.danger {
		color: var(--error);
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 0.3rem;
	}
	form h3 {
		margin: 0 0 0.2rem;
		font-size: 0.9rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.8rem;
	}
	.actions {
		display: flex;
		gap: 0.4rem;
		margin-top: 0.2rem;
	}
</style>
