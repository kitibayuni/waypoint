<script lang="ts">
	import { onDestroy } from 'svelte';
	import cytoscape from 'cytoscape';
	import type { Core, ElementDefinition } from 'cytoscape';

	let {
		elements,
		onHostDblClick
	}: {
		elements: ElementDefinition[];
		onHostDblClick?: (hostId: string) => void;
	} = $props();

	let container: HTMLDivElement;
	let cy: Core | null = null;

	function rebuild() {
		cy?.destroy();
		cy = cytoscape({
			container,
			elements,
			style: [
				{
					selector: 'node',
					style: {
						label: 'data(label)',
						color: '#fff',
						'text-valign': 'center',
						'text-halign': 'center',
						'font-size': '10px',
						width: 'label',
						height: 'label',
						padding: '10px',
						shape: 'round-rectangle'
					}
				},
				{ selector: 'node[type = "host"]', style: { 'background-color': '#3b6fa0' } },
				{ selector: 'node[type = "credential"]', style: { 'background-color': '#a0663b' } },
				{ selector: 'node[type = "observation"]', style: { 'background-color': '#a03b3b' } },
				{ selector: 'node[type = "technique"]', style: { 'background-color': '#6a3ba0' } },
				{
					selector: 'edge',
					style: {
						label: 'data(label)',
						'font-size': '8px',
						width: 2,
						'line-color': '#bbb',
						'target-arrow-color': '#bbb',
						'target-arrow-shape': 'triangle',
						'curve-style': 'bezier'
					}
				},
				{
					selector: 'edge[type = "attack-path"]',
					style: {
						'line-color': '#6a3ba0',
						'target-arrow-color': '#6a3ba0',
						'line-style': 'dashed'
					}
				},
				{
					selector: 'edge[type = "cred-reuse"]',
					style: { 'line-color': '#a0663b', 'target-arrow-color': '#a0663b' }
				},
				{
					selector: 'edge[type = "trust"]',
					style: { 'line-color': '#3b6fa0', 'target-arrow-color': '#3b6fa0' }
				}
			],
			layout: { name: 'cose', animate: false }
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
