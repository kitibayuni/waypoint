<script lang="ts">
	import { calculateCvss } from '$lib/cvss';
	import type { CvssMetrics, CvssResult } from '$lib/cvss';

	let { onApply }: { onApply: (result: CvssResult) => void } = $props();

	let metrics = $state<CvssMetrics>({
		AV: 'N',
		AC: 'L',
		PR: 'N',
		UI: 'N',
		S: 'U',
		C: 'N',
		I: 'N',
		A: 'N'
	});

	const result = $derived(calculateCvss(metrics));
</script>

<div class="cvss-calc">
	<div class="grid">
		<label>
			Attack Vector
			<select bind:value={metrics.AV}>
				<option value="N">Network</option>
				<option value="A">Adjacent</option>
				<option value="L">Local</option>
				<option value="P">Physical</option>
			</select>
		</label>
		<label>
			Attack Complexity
			<select bind:value={metrics.AC}>
				<option value="L">Low</option>
				<option value="H">High</option>
			</select>
		</label>
		<label>
			Privileges Required
			<select bind:value={metrics.PR}>
				<option value="N">None</option>
				<option value="L">Low</option>
				<option value="H">High</option>
			</select>
		</label>
		<label>
			User Interaction
			<select bind:value={metrics.UI}>
				<option value="N">None</option>
				<option value="R">Required</option>
			</select>
		</label>
		<label>
			Scope
			<select bind:value={metrics.S}>
				<option value="U">Unchanged</option>
				<option value="C">Changed</option>
			</select>
		</label>
		<label>
			Confidentiality
			<select bind:value={metrics.C}>
				<option value="N">None</option>
				<option value="L">Low</option>
				<option value="H">High</option>
			</select>
		</label>
		<label>
			Integrity
			<select bind:value={metrics.I}>
				<option value="N">None</option>
				<option value="L">Low</option>
				<option value="H">High</option>
			</select>
		</label>
		<label>
			Availability
			<select bind:value={metrics.A}>
				<option value="N">None</option>
				<option value="L">Low</option>
				<option value="H">High</option>
			</select>
		</label>
	</div>
	<div class="result">
		<span class="score severity-{result.severity}">{result.score.toFixed(1)}</span>
		<span class="severity-label">{result.severity}</span>
		<code class="vector">{result.vector}</code>
		<button type="button" onclick={() => onApply(result)}>Apply to finding</button>
	</div>
</div>

<style>
	.cvss-calc {
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.75rem;
		margin-bottom: 1rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		font-size: 0.8rem;
	}
	.result {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		flex-wrap: wrap;
	}
	.score {
		font-size: 1.3rem;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
	}
	.severity-label {
		font-size: 0.75rem;
		text-transform: uppercase;
		color: #777;
	}
	.vector {
		font-size: 0.8rem;
		background: #f5f5f5;
		padding: 0.15rem 0.4rem;
		border-radius: 4px;
	}
	.severity-none {
		color: #898781;
	}
	.severity-low {
		color: #0ca30c;
	}
	.severity-medium {
		color: #fab219;
	}
	.severity-high {
		color: #ec835a;
	}
	.severity-critical {
		color: #d03b3b;
	}
</style>
