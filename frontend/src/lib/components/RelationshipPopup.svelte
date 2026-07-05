<script lang="ts">
	import { createTrustRelationship } from '$lib/api/trust_relationships';
	import { clampToViewport } from '$lib/actions/clampToViewport';

	let {
		info,
		engagementId,
		onClose,
		onChanged
	}: {
		info: { fromHostId: string; toHostId: string; x: number; y: number };
		engagementId: string;
		onClose: () => void;
		onChanged: () => void;
	} = $props();

	let popupEl = $state<HTMLElement>();
	let kind = $state('session');
	let note = $state('');
	let error = $state('');
	let listenForOutsideClicks = $state(false);

	// The mouseup that ends the edgehandles drag also fires a native `click` on
	// the canvas (mousedown and mouseup share the same target element -- the
	// canvas -- even though the pointer moved a lot in between). That click
	// would otherwise reach this component's own document listener the instant
	// it mounts and immediately close it. Defer attaching until the next tick,
	// after that originating click has already finished bubbling.
	$effect(() => {
		const timer = setTimeout(() => (listenForOutsideClicks = true), 0);
		return () => clearTimeout(timer);
	});

	function handleDocumentClick(e: MouseEvent) {
		if (!listenForOutsideClicks) return;
		const target = e.target as Node;
		if (popupEl && !popupEl.contains(target)) onClose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	async function handleConfirm(e: SubmitEvent) {
		e.preventDefault();
		try {
			await createTrustRelationship(engagementId, {
				from_host_id: info.fromHostId,
				to_host_id: info.toHostId,
				kind,
				note: note || null
			});
			onChanged();
			onClose();
		} catch {
			error = 'Failed to add relationship.';
		}
	}
</script>

<svelte:document onclick={handleDocumentClick} onkeydown={handleKeydown} />

<form class="popup" bind:this={popupEl} use:clampToViewport={{ x: info.x, y: info.y }} onsubmit={handleConfirm}>
	<h3>Add relationship</h3>
	{#if error}
		<p class="error">{error}</p>
	{/if}
	<label>
		Kind
		<select bind:value={kind}>
			<option value="session">session</option>
			<option value="domain_trust">domain trust</option>
			<option value="admin_of">admin of</option>
			<option value="shares_creds">shares creds</option>
		</select>
	</label>
	<label>
		Note
		<input bind:value={note} placeholder="note (optional)" />
	</label>
	<div class="actions">
		<button type="submit">Confirm</button>
		<button type="button" onclick={onClose}>Cancel</button>
	</div>
</form>

<style>
	.popup {
		position: fixed;
		z-index: 60;
		min-width: 12rem;
		background: var(--surface);
		border: 1px solid var(--border-strong);
		border-radius: 6px;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
		padding: 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.popup h3 {
		margin: 0 0 0.2rem;
		font-size: 0.9rem;
	}
	.error {
		color: var(--error);
		font-size: 0.8rem;
		margin: 0;
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
