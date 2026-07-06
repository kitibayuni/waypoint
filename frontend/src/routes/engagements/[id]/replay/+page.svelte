<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onDestroy, onMount, tick } from 'svelte';
	import type { ElementDefinition } from 'cytoscape';
	import { getGraph } from '$lib/api/graph';
	import { getTimeline } from '$lib/api/timeline';
	import type { TimelineEvent } from '$lib/api/timeline';
	import { asOfTimestamp } from '$lib/stores/replay';
	import AttackGraph from '$lib/components/AttackGraph.svelte';

	const engagementId = $page.params.id as string;

	// Playback speed is a multiplier over BASE_INTERVAL_MS (the per-event delay
	// at 1x) -- higher speed = shorter delay between auto-advanced events.
	// Default is faster than a plain 1x so replaying a busy engagement doesn't
	// feel sluggish; still adjustable live via the speed selector.
	const SPEED_OPTIONS = [0.5, 1, 2, 4, 8];
	const BASE_INTERVAL_MS = 1600;
	let speed = $state(4);

	let elements = $state<ElementDefinition[]>([]);
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

	// The scrub bar is positioned by real elapsed time (not event index), like a
	// video timeline -- events cluster realistically instead of being evenly
	// spaced, and dragging snaps to whichever event is closest to that instant.
	const startMs = $derived(events.length ? new Date(events[0].at).getTime() : 0);
	const totalSpanMs = $derived(
		events.length > 1 ? new Date(events[events.length - 1].at).getTime() - startMs : 0
	);

	function eventOffsetMs(i: number): number {
		return new Date(events[i].at).getTime() - startMs;
	}

	// Percent position along the track for event i, falling back to even
	// index-based spacing only in the degenerate case where every event shares
	// the same timestamp (totalSpanMs would otherwise divide by zero).
	function pctFor(i: number): number {
		if (totalSpanMs > 0) return (eventOffsetMs(i) / totalSpanMs) * 100;
		return events.length > 1 ? (i / (events.length - 1)) * 100 : 0;
	}

	function nearestIndexForOffset(offsetMs: number): number {
		let best = 0;
		let bestDiff = Infinity;
		events.forEach((ev, i) => {
			const diff = Math.abs(new Date(ev.at).getTime() - startMs - offsetMs);
			if (diff < bestDiff) {
				bestDiff = diff;
				best = i;
			}
		});
		return best;
	}

	async function loadGraphAsOf(at: string) {
		try {
			const graph = await getGraph(engagementId, at);
			elements = [...graph.nodes, ...graph.edges] as ElementDefinition[];
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
		setIndex(nearestIndexForOffset(Number(target.value)));
	}

	function handleFeedClick(i: number) {
		stopPlaying();
		setIndex(i);
	}

	function advanceOrStop() {
		if (index >= events.length - 1) {
			stopPlaying();
			return;
		}
		setIndex(index + 1);
	}

	function startTimer() {
		if (playTimer) clearInterval(playTimer);
		playTimer = setInterval(advanceOrStop, BASE_INTERVAL_MS / speed);
	}

	function togglePlay() {
		if (playing) {
			stopPlaying();
			return;
		}
		playing = true;
		startTimer();
	}

	function stopPlaying() {
		playing = false;
		if (playTimer) {
			clearInterval(playTimer);
			playTimer = null;
		}
	}

	// Live-adjust the running interval when speed changes mid-playback, rather
	// than only applying the new speed the next time Play is pressed.
	$effect(() => {
		void speed;
		if (playing) startTimer();
	});

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
	});

	onDestroy(stopPlaying);
</script>

<main>
	<h1>Replay</h1>
	<p class="muted">Double-click a host node to open its host page.</p>

	{#if error}
		<p class="error">{error}</p>
	{/if}

	{#if loading}
		<p>Loading…</p>
	{:else if events.length === 0}
		<p class="muted">No timeline events yet — add hosts or findings to see a replay.</p>
	{:else}
		<div class="timeline-bar">
			<button type="button" class="play-btn" onclick={togglePlay} aria-label={playing ? 'Pause' : 'Play'}>
				{playing ? '⏸' : '▶'}
			</button>
			<div class="track-wrap">
				<div class="track-visual">
					<div class="track-fill" style={`width: ${pctFor(index)}%`}></div>
					{#each events as ev, i (ev.subject_type + ev.subject_id + ev.at + ev.event_type)}
						<button
							type="button"
							class="marker"
							class:passed={i <= index}
							style={`left: ${pctFor(i)}%`}
							onclick={() => handleFeedClick(i)}
							title={`${ev.title} — ${new Date(ev.at).toLocaleString()}`}
							aria-label={ev.title}
						></button>
					{/each}
				</div>
				<input
					class="scrub-input"
					type="range"
					min="0"
					max={Math.max(totalSpanMs, 1)}
					value={eventOffsetMs(index)}
					oninput={handleScrub}
					aria-label="Replay position"
				/>
			</div>
			<div class="time-label">
				<span>{new Date(events[index].at).toLocaleString()}</span>
				<span class="muted">Day {currentDay} of {totalDays}</span>
			</div>
			<label class="speed-control">
				Speed
				<select bind:value={speed}>
					{#each SPEED_OPTIONS as s (s)}
						<option value={s}>{s}×</option>
					{/each}
				</select>
			</label>
		</div>

		<div class="layout">
			<AttackGraph
				{elements}
				onHostDblClick={(hostId) => {
					stopPlaying();
					goto(`/engagements/${engagementId}/hosts/${hostId}`);
				}}
				positions={{ engagementId }}
			/>
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
	{/if}
</main>

<style>
	.error {
		color: var(--error);
	}
	.muted {
		color: var(--text-muted);
	}
	.layout {
		display: grid;
		grid-template-columns: 1fr 22rem;
		grid-template-rows: minmax(0, 1fr);
		gap: 1rem;
		height: 65vh;
	}
	.timeline-bar {
		display: flex;
		align-items: center;
		gap: 0.85rem;
		margin: 0.75rem 0 1.25rem;
		padding: 0.6rem 0.85rem;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface);
	}
	.play-btn {
		flex: none;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: 50%;
		border: 1px solid var(--border-strong);
		background: var(--surface-2);
		cursor: pointer;
		font-size: 1rem;
		line-height: 1;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.play-btn:hover {
		border-color: var(--accent);
	}
	.track-wrap {
		position: relative;
		flex: 1 1 auto;
		height: 1.5rem;
		display: flex;
		align-items: center;
		min-width: 0;
	}
	.track-visual {
		position: absolute;
		left: 0;
		right: 0;
		height: 4px;
		border-radius: 999px;
		background: var(--surface-2);
		pointer-events: none;
	}
	.track-fill {
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		border-radius: 999px;
		background: var(--accent);
	}
	.marker {
		position: absolute;
		top: 50%;
		width: 8px;
		height: 8px;
		transform: translate(-50%, -50%);
		border-radius: 50%;
		background: var(--text-muted);
		border: 1px solid var(--surface);
		padding: 0;
		cursor: pointer;
		pointer-events: auto;
	}
	.marker.passed {
		background: var(--accent);
	}
	.scrub-input {
		position: relative;
		z-index: 1;
		width: 100%;
		margin: 0;
		background: transparent;
		appearance: none;
		-webkit-appearance: none;
	}
	.scrub-input::-webkit-slider-runnable-track {
		background: transparent;
		height: 4px;
	}
	.scrub-input::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 14px;
		height: 14px;
		margin-top: -5px;
		border-radius: 50%;
		background: var(--accent);
		border: 2px solid var(--surface);
		cursor: pointer;
	}
	.scrub-input::-moz-range-track {
		background: transparent;
		height: 4px;
	}
	.scrub-input::-moz-range-thumb {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--accent);
		border: 2px solid var(--surface);
		cursor: pointer;
	}
	.time-label {
		flex: none;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		font-size: 0.8rem;
		white-space: nowrap;
	}
	.speed-control {
		flex: none;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8rem;
		color: var(--text-muted);
	}
	.speed-control select {
		font: inherit;
	}
	.feed {
		height: 100%;
		overflow-y: auto;
		border: 1px solid var(--border);
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
		border-color: var(--accent);
		background: var(--surface-2);
	}
	.feed li.future button {
		opacity: 0.4;
	}
	.event-type {
		font-size: 0.7rem;
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.summary {
		font-size: 0.8rem;
		color: var(--text-muted);
	}
	.feed time {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
</style>
