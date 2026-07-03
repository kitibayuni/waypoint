<script lang="ts">
	import { page } from '$app/stores';
	import { onMount, onDestroy, tick } from 'svelte';
	import cytoscape from 'cytoscape';
	import type { Core, ElementDefinition } from 'cytoscape';
	import { getGraph } from '$lib/api/graph';
	import { getTimeline } from '$lib/api/timeline';
	import type { TimelineEvent } from '$lib/api/timeline';
	import { asOfTimestamp } from '$lib/stores/replay';

	const engagementId = $page.params.id as string;

	let container: HTMLDivElement;
	let cy: Core | null = null;
	let events = $state<TimelineEvent[]>([]);
	let feedRefs: (HTMLLIElement | null)[] = [];
	let index = $state(0);
	let playing = $state(false);
	let loading = $state(true);
	let error = $state('');
	let playTimer: ReturnType<typeof setInterval> | null = null;

	const totalDays = $derived(
		events.length > 1
			? Math.max(
					1,
					Math.ceil(
						(new Date(events[events.length - 1].at).getTime() - new Date(events[0].at).getTime()) /
							86_400_000
					) + 1
				)
			: 1
	);

	const currentDay = $derived(
		events.length > 0
			? Math.floor(
					(new Date(events[index].at).getTime() - new Date(events[0].at).getTime()) / 86_400_000
				) + 1
			: 1
	);

	async function loadGraphAsOf(at: string) {
		try {
			const graph = await getGraph(engagementId, at);
			cy?.destroy();
			cy = cytoscape({
				container,
				elements: [...graph.nodes, ...graph.edges] as ElementDefinition[],
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
		} catch {
			error = 'Failed to load attack graph.';
		}
	}

	async function setIndex(i: number) {
		if (events.length === 0) return;
		index = Math.max(0, Math.min(events.length - 1, i));
		const at = events[index].at;
		asOfTimestamp.set(at);
		await loadGraphAsOf(at);
		await tick();
		feedRefs[index]?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
	}

	function handleScrub(e: Event) {
		const target = e.target as HTMLInputElement;
		stopPlaying();
		setIndex(Number(target.value));
	}

	function handleFeedClick(i: number) {
		stopPlaying();
		setIndex(i);
	}

	function togglePlay() {
		if (playing) {
			stopPlaying();
			return;
		}
		playing = true;
		playTimer = setInterval(() => {
			if (index >= events.length - 1) {
				stopPlaying();
				return;
			}
			setIndex(index + 1);
		}, 1500);
	}

	function stopPlaying() {
		playing = false;
		if (playTimer) {
			clearInterval(playTimer);
			playTimer = null;
		}
	}

	async function load() {
		loading = true;
		error = '';
		try {
			events = await getTimeline(engagementId);
			if (events.length > 0) {
				await setIndex(events.length - 1);
			} else {
				await loadGraphAsOf(new Date().toISOString());
			}
		} catch {
			error = 'Failed to load timeline.';
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		load();
		return () => {
			stopPlaying();
			cy?.destroy();
		};
	});

	onDestroy(stopPlaying);
</script>

<main>
	<p><a href={`/engagements/${engagementId}`}>&larr; Engagement overview</a></p>
	<h1>Replay</h1>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	{#if loading}
		<p>Loading…</p>
	{:else if events.length === 0}
		<p class="muted">No timeline events yet — add hosts, observations, or findings to see a replay.</p>
	{/if}

	<div class="layout" class:hidden={loading || events.length === 0}>
		<div class="graph-container" bind:this={container}></div>
		<aside class="feed">
			<h2>Feed</h2>
			<ul>
				{#each events as ev, i (ev.subject_type + ev.subject_id + ev.at + ev.event_type)}
					<li
						bind:this={feedRefs[i]}
						class:active={i === index}
						class:future={i > index}
					>
						<button type="button" onclick={() => handleFeedClick(i)}>
							<span class="event-type">{ev.event_type.replaceAll('_', ' ')}</span>
							<strong>{ev.title}</strong>
							{#if ev.summary}
								<span class="summary">{ev.summary}</span>
							{/if}
							<time>{new Date(ev.at).toLocaleString()}</time>
						</button>
					</li>
				{/each}
			</ul>
		</aside>
	</div>

	{#if !loading && events.length > 0}
		<div class="scrubber">
			<input
				type="range"
				min="0"
				max={events.length - 1}
				value={index}
				oninput={handleScrub}
			/>
			<div class="scrubber-controls">
				<button type="button" onclick={togglePlay}>{playing ? '⏸ Pause' : '▶ Play'}</button>
				<span>Day {currentDay} of {totalDays}</span>
				<span class="muted">{new Date(events[index].at).toLocaleString()}</span>
			</div>
		</div>
	{/if}
</main>

<style>
	.error {
		color: #c0392b;
	}
	.muted {
		color: #777;
	}
	.layout {
		display: grid;
		grid-template-columns: 1fr 22rem;
		gap: 1rem;
		height: 65vh;
	}
	.layout.hidden {
		display: none;
	}
	.graph-container {
		height: 100%;
		border: 1px solid #ddd;
		border-radius: 6px;
		background: #fafafa;
	}
	.feed {
		height: 100%;
		overflow-y: auto;
		border: 1px solid #ddd;
		border-radius: 6px;
		padding: 0.75rem;
		box-sizing: border-box;
	}
	.feed ul {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.feed li button {
		width: 100%;
		text-align: left;
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		background: none;
		border: 1px solid transparent;
		border-radius: 4px;
		padding: 0.4rem 0.5rem;
		cursor: pointer;
		font: inherit;
	}
	.feed li.active button {
		border-color: #333;
		background: #f0f0f0;
	}
	.feed li.future button {
		opacity: 0.4;
	}
	.event-type {
		font-size: 0.7rem;
		text-transform: uppercase;
		color: #888;
	}
	.summary {
		font-size: 0.8rem;
		color: #555;
	}
	.feed time {
		font-size: 0.75rem;
		color: #999;
	}
	.scrubber {
		margin-top: 1rem;
	}
	.scrubber input[type='range'] {
		width: 100%;
	}
	.scrubber-controls {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-top: 0.4rem;
		font-size: 0.85rem;
	}
</style>
