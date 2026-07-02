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
		border: 1px solid #ccc;
		border-radius: 999px;
		padding: 0.25rem 0.7rem;
		font-size: 0.85rem;
		cursor: pointer;
		background: #fafafa;
	}
	.status-confirmed {
		border-color: #c0392b;
		background: #fdecea;
	}
	.status-suspected {
		border-color: #d4a017;
		background: #fff8e1;
	}
	.status-remediated {
		border-color: #27ae60;
		background: #eafaf1;
	}
	.status-false_positive {
		border-color: #999;
		background: #f0f0f0;
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
