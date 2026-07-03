<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { currentUser, authChecked, refreshMe } from '$lib/stores/auth';

	let { children } = $props();

	onMount(async () => {
		await refreshMe();
	});

	$effect(() => {
		if (!$authChecked) return;
		const path = $page.url.pathname;
		if (!$currentUser && path !== '/login') {
			goto('/login');
		} else if ($currentUser && path === '/login') {
			goto('/');
		}
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{#if $authChecked}
	{@render children()}
{/if}
