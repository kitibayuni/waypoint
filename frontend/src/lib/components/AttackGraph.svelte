<script lang="ts">
	import { onDestroy } from 'svelte';
	import cytoscape from 'cytoscape';
	import type { Core, ElementDefinition } from 'cytoscape';

	let {
		elements,
		onHostDblClick,
		compact = false,
		interactive = true
	}: {
		elements: ElementDefinition[];
		onHostDblClick?: (hostId: string) => void;
		/** Smaller fonts/padding and tighter layout spacing, for the Dashboard mini-graph panel. */
		compact?: boolean;
		/** Set false to disable zoom/pan/box-selection, e.g. for a read-only overview panel. */
		interactive?: boolean;
	} = $props();

	let container: HTMLDivElement;
	let cy: Core | null = null;

	// The Dashboard mini-graph and the full-size attack graph/Hosts-map/Replay
	// views share one visual language (node/edge colors, the foothold
	// highlight) but need different scale and layout tightness for their very
	// different container sizes -- `compact` switches between the two
	// tunings here instead of each consumer hand-duplicating this whole
	// style+layout block (which is what happened before this was factored out).
	// cytoscape's style-array type doesn't unify well across conditionally
	// different-shaped rule objects (compact vs. full edge styling); the
	// object shapes themselves are still exactly what cytoscape expects.
	function buildStyle(): any[] {
		const styles: any[] = [
			{
				selector: 'node',
				style: {
					label: 'data(label)',
					color: '#fff',
					'text-valign': 'center',
					'text-halign': 'center',
					'text-wrap': 'wrap',
					'text-max-width': compact ? '60px' : '90px',
					'font-size': compact ? '7px' : '10px',
					width: 'label',
					height: 'label',
					padding: compact ? '6px' : '10px',
					shape: 'round-rectangle'
				}
			},
			{ selector: 'node[type = "host"]', style: { 'background-color': '#3b6fa0' } },
			{ selector: 'node[type = "host"][?is_foothold]', style: { 'background-color': '#e04343' } },
			{ selector: 'node[type = "credential"]', style: { 'background-color': '#a0663b' } },
			compact
				? {
						selector: 'edge',
						style: {
							width: 1,
							color: '#fff',
							'line-color': '#5a6270',
							'curve-style': 'bezier',
							'control-point-step-size': 40
						}
					}
				: {
						selector: 'edge',
						style: {
							label: 'data(label)',
							color: '#fff',
							'font-size': '8px',
							'text-rotation': 'autorotate',
							'text-background-color': '#14161c',
							'text-background-opacity': 0.85,
							'text-background-padding': '2px',
							'text-background-shape': 'roundrectangle',
							width: 2,
							'line-color': '#bbb',
							'target-arrow-color': '#bbb',
							'target-arrow-shape': 'triangle',
							'curve-style': 'bezier',
							'control-point-step-size': 60
						}
					}
		];

		if (!compact) {
			styles.push(
				{
					selector: 'edge[type = "cred-reuse"]',
					style: { 'line-color': '#a0663b', 'target-arrow-color': '#a0663b' }
				},
				{
					selector: 'edge[type = "trust"]',
					style: { 'line-color': '#3b6fa0', 'target-arrow-color': '#3b6fa0' }
				}
			);
		}

		return styles;
	}

	function buildLayout() {
		return compact
			? {
					name: 'cose' as const,
					animate: false,
					fit: true,
					padding: 20,
					nodeDimensionsIncludeLabels: true,
					componentSpacing: 80,
					idealEdgeLength: () => 30,
					edgeElasticity: () => 100,
					gravity: 60,
					numIter: 2000,
					nodeOverlap: 10
				}
			: {
					name: 'cose' as const,
					animate: false,
					fit: true,
					padding: 40,
					nodeDimensionsIncludeLabels: true,
					componentSpacing: 100,
					nodeRepulsion: () => 12000,
					idealEdgeLength: () => 90,
					edgeElasticity: () => 200,
					gravity: 30,
					numIter: 3000,
					nodeOverlap: 20
				};
	}

	function rebuild() {
		cy?.destroy();
		cy = cytoscape({
			container,
			elements,
			style: buildStyle(),
			layout: buildLayout(),
			userZoomingEnabled: interactive,
			userPanningEnabled: interactive,
			boxSelectionEnabled: interactive
		});

		cy.on('dbltap', 'node[type = "host"]', (evt) => {
			const id = evt.target.id() as string;
			onHostDblClick?.(id.replace(/^host:/, ''));
		});
	}

	$effect(() => {
		// Re-run whenever the `elements` prop reference changes.
		void elements;
		if (container) rebuild();
	});

	onDestroy(() => cy?.destroy());
</script>

<div class="graph-container" bind:this={container}></div>

<style>
	/*
	 * `.graph-container` sits in a CSS-grid cell with no explicit row height
	 * (see the parent `.layout` in graph/+page.svelte and replay/+page.svelte).
	 * Grid items default to min-height/min-width: auto, so without the
	 * min-height/min-width: 0 + overflow: hidden below, cytoscape's internal
	 * canvas-container div (an in-flow child, sized via inline styles derived
	 * from the *previous* measurement) can inflate this box on every rebuild,
	 * compounding indefinitely across repeated destroy/recreate cycles
	 * (e.g. every replay autoplay tick). position: relative establishes the
	 * containing block cytoscape's own absolutely-positioned canvas layers
	 * expect.
	 */
	.graph-container {
		height: 100%;
		width: 100%;
		min-height: 0;
		min-width: 0;
		position: relative;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
	}
</style>
