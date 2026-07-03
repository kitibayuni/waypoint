<script lang="ts">
	import type { Observation } from '$lib/api/observations';

	let { observation, onclick }: { observation: Observation; onclick?: () => void } = $props();

	const severity = $derived(observation.severity_override ?? observation.default_severity);
</script>

<button type="button" class="chip status-{observation.status}" {onclick}>
	<span class="title">{observation.observation_title}</span>
	<span class="severity">{severity}</span>
	<span class="status-label">{observation.status.replace('_', ' ')}</span>
</button>

<style>
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		padding: 0.25rem 0.7rem;
		font-size: 0.85rem;
		cursor: pointer;
		background: var(--surface);
	}
	.status-confirmed {
		border-color: var(--error);
		background: var(--error-bg);
	}
	.status-suspected {
		border-color: var(--warning);
		background: var(--warning-bg);
	}
	.status-remediated {
		border-color: var(--success);
		background: var(--success-bg);
	}
	.status-false_positive {
		border-color: var(--text-muted);
		background: var(--surface-2);
		text-decoration: line-through;
		opacity: 0.7;
	}
	.severity,
	.status-label {
		font-size: 0.7rem;
		text-transform: uppercase;
		opacity: 0.75;
	}
</style>
