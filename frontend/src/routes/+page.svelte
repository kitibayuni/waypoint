<script lang="ts">
	import { currentUser, logout } from '$lib/stores/auth';
	import { goto } from '$app/navigation';

	async function handleLogout() {
		await logout();
		goto('/login');
	}
</script>

<main>
	{#if $currentUser}
		<h1>Engagement Manager</h1>
		<p>Signed in as {$currentUser.display_name} ({$currentUser.email})</p>
		<button onclick={handleLogout}>Sign out</button>
	{/if}
</main>
