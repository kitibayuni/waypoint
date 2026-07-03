<script lang="ts">
	import { page } from '$app/stores';

	let { children } = $props();

	const engagementId = $derived($page.params.id as string);
</script>

<div class="engagement-shell">
	<nav class="engagement-nav">
		<a href="/engagements">&larr; All engagements</a>
		<a href={`/engagements/${engagementId}`}>Attack graph</a>
		<a href={`/engagements/${engagementId}/overview`}>Overview</a>
		<a href={`/engagements/${engagementId}/hosts`}>Hosts</a>
		<a href={`/engagements/${engagementId}/credentials`}>Credentials</a>
		<a href={`/engagements/${engagementId}/findings`}>Findings</a>
		<a href={`/engagements/${engagementId}/templates`}>Templates</a>
		<a href={`/engagements/${engagementId}/search`}>Search</a>
		<a href={`/engagements/${engagementId}/import`}>Import</a>
		<a href={`/engagements/${engagementId}/replay`}>Replay</a>
	</nav>
	{@render children()}
</div>

<style>
	.engagement-nav {
		display: flex;
		gap: 1rem;
		flex-wrap: wrap;
		padding: 0.6rem 0;
		margin-bottom: 1rem;
		border-bottom: 1px solid var(--border);
		font-size: 0.9rem;
	}
	.engagement-nav a {
		white-space: nowrap;
	}
</style>
