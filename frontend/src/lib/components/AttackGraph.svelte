<script lang="ts">
	import { onDestroy } from 'svelte';
	import cytoscape from 'cytoscape';
	import type { Core, ElementDefinition } from 'cytoscape';
	import { listNodePositions, putNodePositions } from '$lib/api/node_positions';

	let {
		elements,
		onHostDblClick,
		onNodeSelect,
		onContextMenu,
		compact = false,
		interactive = true,
		positions
	}: {
		elements: ElementDefinition[];
		onHostDblClick?: (hostId: string) => void;
		/** Fires on single-click/tap of a node, and with `null` when the selection is cleared. */
		onNodeSelect?: (
			info: { id: string; type: string; data: Record<string, unknown> } | null
		) => void;
		/** Fires on right-click of a node or the graph background (native browser menu is suppressed). */
		onContextMenu?: (info: {
			x: number;
			y: number;
			target: 'background' | 'host' | 'credential';
			nodeId?: string;
		}) => void;
		/** Smaller fonts/padding and tighter layout spacing, for the Dashboard mini-graph panel. */
		compact?: boolean;
		/** Set false to disable zoom/pan/box-selection, e.g. for a read-only overview panel. */
		interactive?: boolean;
		/**
		 * Enables shared, stable node positions for this engagement instead of a fresh
		 * randomized layout every time `elements` changes -- see `rebuild()`. Pass
		 * `persist: true` only from the pages that should be allowed to write dragged/
		 * newly-placed positions back (the root Attack Graph page and the Hosts map);
		 * Replay and the Dashboard mini-graph read the same shared layout but never
		 * write to it.
		 */
		positions?: { engagementId: string; persist?: boolean };
	} = $props();

	let container: HTMLDivElement;
	let cy: Core | null = null;

	let savedPositions: Record<string, { x: number; y: number }> = {};
	let positionsLoadedFor = '';
	let selectedId: string | null = null;
	let pendingChanges: Record<string, { x: number; y: number }> = {};
	let flushTimer: ReturnType<typeof setTimeout> | null = null;
	let resizeObserver: ResizeObserver | null = null;
	let resizeTimer: ReturnType<typeof setTimeout> | null = null;

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
			// Pivot (orange) is declared before foothold (red) so a host that is both
			// renders as foothold -- initial access is the higher-priority signal.
			{ selector: 'node[type = "host"][?is_pivot]', style: { 'background-color': '#d9891f' } },
			{ selector: 'node[type = "host"][?is_foothold]', style: { 'background-color': '#e04343' } },
			// Credential ("user") nodes: translucent black fill with a white outline,
			// rather than a flat color, to read visually distinct from host nodes.
			{
				selector: 'node[type = "credential"]',
				style: {
					'background-color': '#000000',
					'background-opacity': 0.55,
					'border-width': 2,
					'border-style': 'solid',
					'border-color': '#ffffff'
				}
			},
			// Border rather than background, since background already encodes
			// host/credential/foothold identity -- matches the --warning design token.
			{
				selector: 'node.selected',
				style: { 'border-width': 3, 'border-style': 'solid', 'border-color': '#e0b23f' }
			},
			compact
				? {
						selector: 'edge',
						style: {
							width: 1.5,
							'line-color': '#bbb',
							'target-arrow-color': '#bbb',
							'target-arrow-shape': 'triangle',
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
					},
			// Same trust/cred-reuse color language in both modes -- compact only
			// drops the text labels for space, not the rest of the visual identity.
			{
				selector: 'edge[type = "cred-reuse"]',
				style: { 'line-color': '#a0663b', 'target-arrow-color': '#a0663b' }
			},
			{
				selector: 'edge[type = "trust"]',
				style: { 'line-color': '#3b6fa0', 'target-arrow-color': '#3b6fa0' }
			}
		];

		return styles;
	}

	function buildLayout(randomize: boolean) {
		return compact
			? {
					name: 'cose' as const,
					animate: false,
					fit: true,
					randomize,
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
					randomize,
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

	function commitPosition(id: string, pos: { x: number; y: number }) {
		savedPositions = { ...savedPositions, [id]: pos };
		if (positions?.persist && positions.engagementId) queuePersist(id, pos);
	}

	function queuePersist(nodeId: string, pos: { x: number; y: number }) {
		pendingChanges[nodeId] = pos;
		if (flushTimer) clearTimeout(flushTimer);
		flushTimer = setTimeout(flush, 600);
	}

	function flush() {
		flushTimer = null;
		const engId = positions?.engagementId;
		const changes = pendingChanges;
		pendingChanges = {};
		if (!engId || Object.keys(changes).length === 0) return;
		putNodePositions(
			engId,
			Object.entries(changes).map(([node_id, p]) => ({ node_id, ...p }))
		).catch(() => {
			// Best-effort; a transient save failure must not break the graph UI.
		});
	}

	// Nodes with a saved position carry it directly in their element definition so
	// cose's `randomize: false` starts (and settles) from where they already were.
	// Brand-new nodes are seeded near a known neighbor (instead of defaulting to
	// (0,0), which would stack multiple new nodes exactly on top of each other) so
	// cose only really has to place what's actually new. This must be done on the
	// plain element data *before* constructing cytoscape -- an earlier version set
	// positions via `node.position()`/`.lock()` after an initial `{name: 'preset'}`
	// construction pass, but that two-phase approach hit a reproducible cytoscape
	// bug where a subset of nodes silently rendered as non-visible (confirmed via
	// `.visible()` already false immediately after construction, before any of this
	// logic ran) -- baking positions into the elements array for a single direct
	// construction with the real layout avoids it entirely.
	function prepareElements(els: ElementDefinition[]): {
		elements: ElementDefinition[];
		newIds: Set<string>;
	} {
		const newIds = new Set<string>();
		const knownIds: string[] = [];
		const nodeIds = new Set<string>();
		const neighbors: Record<string, string[]> = {};

		for (const el of els) {
			const d = el.data as any;
			if (d?.id && !('source' in d)) {
				nodeIds.add(d.id);
				if (savedPositions[d.id]) knownIds.push(d.id);
				else newIds.add(d.id);
			}
		}
		for (const el of els) {
			const d = el.data as any;
			if (d?.source && d?.target) {
				(neighbors[d.source] ??= []).push(d.target);
				(neighbors[d.target] ??= []).push(d.source);
			}
		}

		const prepared = els.map((el) => {
			const id = (el.data as any)?.id as string | undefined;
			if (!id || !nodeIds.has(id)) return el;
			let pos = savedPositions[id];
			if (!pos) {
				const neighborWithPos = (neighbors[id] ?? []).find((nb) => savedPositions[nb]);
				const base = neighborWithPos
					? savedPositions[neighborWithPos]
					: knownIds.length
						? savedPositions[knownIds[Math.floor(Math.random() * knownIds.length)]]
						: null;
				if (base) {
					pos = { x: base.x + (Math.random() - 0.5) * 120, y: base.y + (Math.random() - 0.5) * 120 };
				}
			}
			// Clone, never hand cytoscape a live reference into savedPositions --
			// cose mutates node position objects in place during its simulation, which
			// would otherwise silently corrupt the saved values through this same
			// reference while the layout that's supposed to respect them is running.
			return pos ? { ...el, position: { x: pos.x, y: pos.y } } : el;
		});

		return { elements: prepared, newIds };
	}

	async function ensurePositionsLoaded() {
		const engId = positions?.engagementId;
		if (!engId) {
			savedPositions = {};
			positionsLoadedFor = '';
			return;
		}
		if (engId === positionsLoadedFor) return;
		const rows = await listNodePositions(engId);
		savedPositions = Object.fromEntries(rows.map((r) => [r.node_id, { x: r.x, y: r.y }]));
		positionsLoadedFor = engId;
	}

	function rebuild() {
		cy?.destroy();
		resizeObserver?.disconnect();
		resizeObserver = null;

		const engId = positions?.engagementId;
		let preparedElements = elements;
		let newIds = new Set<string>();
		let randomize = true;
		if (engId) {
			randomize = Object.keys(savedPositions).length === 0;
			const result = prepareElements(elements);
			preparedElements = result.elements;
			newIds = result.newIds;
		}

		cy = cytoscape({
			container,
			elements: preparedElements,
			style: buildStyle(),
			layout: buildLayout(randomize),
			userZoomingEnabled: interactive,
			userPanningEnabled: interactive,
			boxSelectionEnabled: interactive,
			autoungrabify: !interactive
		});
		const core = cy;

		// Cytoscape never re-observes its container's size on its own -- without this
		// the canvas keeps whatever dimensions it had at construction time even after
		// the window or a split panel resizes, until the next full rebuild.
		resizeObserver = new ResizeObserver(() => {
			if (resizeTimer) clearTimeout(resizeTimer);
			resizeTimer = setTimeout(() => {
				core.resize();
				core.fit(undefined, compact ? 20 : 40);
			}, 120);
		});
		resizeObserver.observe(container);

		core.on('dbltap', 'node[type = "host"]', (evt) => {
			const id = evt.target.id() as string;
			onHostDblClick?.(id.replace(/^host:/, ''));
		});

		if (onNodeSelect) {
			core.on('tap', 'node', (evt) => {
				core.nodes().removeClass('selected');
				const n = evt.target;
				n.addClass('selected');
				selectedId = n.id();
				onNodeSelect({
					id: (n.id() as string).replace(/^(host|credential):/, ''),
					type: n.data('type'),
					data: n.data()
				});
			});
			// Unfiltered `tap` also fires for node taps (delegated up to the core), so
			// only background taps (evt.target === core) should clear the selection.
			core.on('tap', (evt) => {
				if (evt.target === core) {
					core.nodes().removeClass('selected');
					selectedId = null;
					onNodeSelect(null);
				}
			});
			if (selectedId) {
				const existing = core.getElementById(selectedId);
				if (existing.length) {
					existing.addClass('selected');
				} else {
					selectedId = null;
					onNodeSelect(null);
				}
			}
		}

		if (onContextMenu) {
			container.oncontextmenu = (e) => e.preventDefault();
			core.on('cxttap', 'node', (evt) => {
				const original = evt.originalEvent as MouseEvent;
				onContextMenu({
					x: original.clientX,
					y: original.clientY,
					target: evt.target.data('type'),
					nodeId: (evt.target.id() as string).replace(/^(host|credential):/, '')
				});
			});
			core.on('cxttap', (evt) => {
				if (evt.target === core) {
					const original = evt.originalEvent as MouseEvent;
					onContextMenu({ x: original.clientX, y: original.clientY, target: 'background' });
				}
			});
		}

		if (engId) {
			// The construction above already ran the layout synchronously (animate is
			// always false). cose's force simulation can still nudge already-known
			// nodes a little even when seeded at their saved spot (their equilibrium
			// shifts slightly whenever new nodes/edges join the graph), which would
			// make already-placed hosts drift a few pixels on every reload/replay
			// frame. Force them back to their exact saved position now -- keeping
			// only the newly-placed nodes' freshly-computed positions -- then re-fit
			// the viewport to the corrected layout.
			let restoredAny = false;
			core.nodes().forEach((n) => {
				const saved = savedPositions[n.id()];
				if (saved) {
					n.position({ x: saved.x, y: saved.y });
					restoredAny = true;
				}
			});
			if (restoredAny) core.fit(undefined, compact ? 20 : 40);

			newIds.forEach((id) => {
				const p = core.getElementById(id).position();
				commitPosition(id, { x: p.x, y: p.y });
			});
			core.on('dragfree', 'node', (evt) => {
				const p = evt.target.position();
				commitPosition(evt.target.id(), { x: p.x, y: p.y });
			});
		}
	}

	$effect(() => {
		void elements;
		if (!container) return;
		const engId = positions?.engagementId;
		if (engId && engId !== positionsLoadedFor) {
			ensurePositionsLoaded().then(rebuild);
		} else {
			rebuild();
		}
	});

	onDestroy(() => {
		if (flushTimer) {
			clearTimeout(flushTimer);
			flush();
		}
		if (resizeTimer) clearTimeout(resizeTimer);
		resizeObserver?.disconnect();
		cy?.destroy();
	});
</script>

<div class="graph-container" bind:this={container}></div>

<style>
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
