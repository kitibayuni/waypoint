<script lang="ts">
	import { onMount } from 'svelte';
	import type { ElementDefinition } from 'cytoscape';
	import { getDashboard } from '$lib/api/dashboard';
	import type { Dashboard } from '$lib/api/dashboard';
	import { getGraph } from '$lib/api/graph';
	import AttackGraph from '$lib/components/AttackGraph.svelte';

	let { engagementId }: { engagementId: string } = $props();

	let dashboard = $state<Dashboard | null>(null);
	let graphElements = $state<ElementDefinition[]>([]);
	let loading = $state(true);
	let error = $state('');

	// Host status is a workflow progression (an ordinal scale), so it takes
	// one hue with monotone lightness rather than distinct categorical hues.
	const HOST_STAGES = ['discovered', 'enumerating', 'exploited', 'owned', 'cleared'];
	const ORDINAL_RAMP: Record<string, string> = {
		discovered: '#cfe3fb',
		enumerating: '#8fbdf0',
		exploited: '#5b93dc',
		owned: '#3a6ec2',
		cleared: '#24508f'
	};

	// Findings severity is a state/status job (good -> critical), so it wears
	// the fixed, reserved status palette rather than categorical hues.
	const SEVERITY_ORDER = ['critical', 'high', 'medium', 'low'];
	const SEVERITY_STATUS: Record<string, { color: string; role: string }> = {
		critical: { color: 'var(--sev-critical)', role: 'critical' },
		high: { color: 'var(--sev-high)', role: 'serious' },
		medium: { color: 'var(--sev-medium)', role: 'warning' },
		low: { color: 'var(--sev-low)', role: 'good' }
	};

	async function load() {
		loading = true;
		error = '';
		try {
			const [dash, graph] = await Promise.all([getDashboard(engagementId), getGraph(engagementId)]);
			dashboard = dash;
			graphElements = [...graph.nodes, ...graph.edges] as ElementDefinition[];
		} catch {
			error = 'Failed to load dashboard.';
		} finally {
			loading = false;
		}
	}

	onMount(load);

	function maxOf(counts: Record<string, number>): number {
		const values = Object.values(counts);
		return values.length > 0 ? Math.max(...values) : 0;
	}

	function totalOf(counts: Record<string, number>): number {
		return Object.values(counts).reduce((a, b) => a + b, 0);
	}
</script>

<div class="dashboard">
	{#if error}
		<p class="error">{error}</p>
	{/if}

	{#if loading}
		<p>Loading dashboard…</p>
	{:else if dashboard}
		<div class="dashboard-grid">
			<section class="panel graph-panel">
				<h3>Attack graph</h3>
				<div class="mini-graph">
					<AttackGraph
						elements={graphElements}
						compact
						interactive={false}
						positions={{ engagementId }}
					/>
				</div>
			</section>

			<div class="side-stack">
				<div class="kpi-row">
					<div class="stat-tile">
						<span class="stat-value">{totalOf(dashboard.hosts_by_status)}</span>
						<span class="stat-label">Hosts</span>
					</div>
					<div class="stat-tile">
						<span class="stat-value">{dashboard.scope_count}</span>
						<span class="stat-label">Scope items</span>
					</div>
					<div class="stat-tile">
						<span class="stat-value">{dashboard.credentials.total}</span>
						<span class="stat-label">Credentials</span>
						<span class="stat-sub"
							>{dashboard.credentials.validated} validated · {dashboard.credentials.reused} reused</span
						>
					</div>
					<div class="stat-tile">
						<span class="stat-value">{totalOf(dashboard.findings_by_severity)}</span>
						<span class="stat-label">Findings</span>
					</div>
					<div class="stat-tile">
						{#if dashboard.engagement.days_remaining !== null}
							<span class="stat-value">{dashboard.engagement.days_remaining}</span>
							<span class="stat-label">Days remaining</span>
						{:else if dashboard.engagement.days_elapsed !== null}
							<span class="stat-value">{dashboard.engagement.days_elapsed}</span>
							<span class="stat-label">Days elapsed</span>
						{:else}
							<span class="stat-value">—</span>
							<span class="stat-label">No timeline set</span>
						{/if}
					</div>
				</div>

				<section class="panel">
					<h3>Checklist completion</h3>
					<div class="meter" role="meter" aria-valuenow={dashboard.checklist.completion_pct} aria-valuemin="0" aria-valuemax="100">
						<div class="meter-fill" style="width: {dashboard.checklist.completion_pct}%"></div>
					</div>
					<p class="meter-label">
						{dashboard.checklist.completion_pct}% ({dashboard.checklist.done + dashboard.checklist.na} of
						{dashboard.checklist.total} items done or n/a)
					</p>
				</section>

				<section class="panel">
					<h3>Hosts by status</h3>
					{#each HOST_STAGES as stage (stage)}
						{@const count = dashboard.hosts_by_status[stage] ?? 0}
						{@const max = maxOf(dashboard.hosts_by_status) || 1}
						<div class="bar-row">
							<span class="bar-label">{stage}</span>
							<div class="bar-track">
								<div
									class="bar-fill"
									style="width: {(count / max) * 100}%; background: {ORDINAL_RAMP[stage]}"
								></div>
							</div>
							<span class="bar-count">{count}</span>
						</div>
					{/each}
				</section>

				<section class="panel">
					<h3>Findings by severity</h3>
					{#each SEVERITY_ORDER as severity (severity)}
						{@const count = dashboard.findings_by_severity[severity] ?? 0}
						{@const max = maxOf(dashboard.findings_by_severity) || 1}
						<div class="bar-row">
							<span
								class="status-dot"
								style="background: {SEVERITY_STATUS[severity].color}"
								aria-hidden="true"
							></span>
							<span class="bar-label">{severity}</span>
							<div class="bar-track">
								<div
									class="bar-fill"
									style="width: {(count / max) * 100}%; background: {SEVERITY_STATUS[severity].color}"
								></div>
							</div>
							<span class="bar-count">{count}</span>
						</div>
					{/each}
				</section>
			</div>
		</div>
	{/if}
</div>

<style>
	.dashboard {
		--surface-1: var(--surface);
		--text-primary: var(--text);
		--text-secondary: var(--text-muted);
		--muted: var(--text-faint);
		--gridline: var(--border);
		--sequential-400: #3987e5;
		--sequential-150: #29344a;
		margin-bottom: 2rem;
	}
	.error {
		color: var(--error);
	}
	.kpi-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
		gap: 0.75rem;
	}
	.stat-tile {
		background: var(--surface-1);
		border: 1px solid var(--gridline);
		border-radius: 8px;
		padding: 0.9rem;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.stat-value {
		font-size: 1.8rem;
		font-weight: 600;
		color: var(--text-primary);
		font-variant-numeric: proportional-nums;
	}
	.stat-label {
		font-size: 0.8rem;
		color: var(--text-secondary);
	}
	.stat-sub {
		font-size: 0.72rem;
		color: var(--muted);
	}
	.dashboard-grid {
		display: grid;
		grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
		gap: 1rem;
		align-items: start;
	}
	.side-stack {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	@media (max-width: 900px) {
		.dashboard-grid {
			grid-template-columns: 1fr;
		}
	}
	.panel {
		border: 1px solid var(--gridline);
		border-radius: 8px;
		padding: 0.9rem;
	}
	.panel h3 {
		margin: 0 0 0.6rem;
		font-size: 0.95rem;
	}
	.meter {
		height: 0.9rem;
		background: var(--sequential-150);
		border-radius: 999px;
		overflow: hidden;
	}
	.meter-fill {
		height: 100%;
		background: var(--sequential-400);
		border-radius: 999px;
	}
	.meter-label {
		font-size: 0.8rem;
		color: var(--text-secondary);
		margin: 0.4rem 0 0;
	}
	.bar-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.4rem;
		font-size: 0.8rem;
	}
	.bar-label {
		width: 5.5rem;
		flex-shrink: 0;
		color: var(--text-secondary);
	}
	.bar-track {
		flex: 1;
		height: 0.6rem;
		background: var(--gridline);
		border-radius: 4px;
		overflow: hidden;
	}
	.bar-fill {
		height: 100%;
		border-radius: 4px;
	}
	.bar-count {
		width: 1.5rem;
		text-align: right;
		font-variant-numeric: tabular-nums;
		color: var(--text-primary);
	}
	.status-dot {
		display: inline-block;
		width: 0.6rem;
		height: 0.6rem;
		border-radius: 50%;
		flex-shrink: 0;
	}
	.mini-graph {
		/* AttackGraph's own .graph-container supplies the border/background/
		   radius; this wrapper only needs to give it a definite height to
		   fill (see AttackGraph.svelte's height: 100% comment). */
		height: 28rem;
	}
</style>
