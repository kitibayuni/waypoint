<script lang="ts">
	import { goto } from '$app/navigation';
	import { login } from '$lib/stores/auth';

	let email = $state('');
	let password = $state('');
	let error = $state('');
	let submitting = $state(false);

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		submitting = true;
		try {
			await login(email, password);
			goto('/');
		} catch {
			error = 'Invalid email or password.';
		} finally {
			submitting = false;
		}
	}
</script>

<main class="login">
	<form onsubmit={handleSubmit}>
		<h1>Sign in</h1>
		<label>
			Email
			<input type="email" bind:value={email} required autocomplete="username" />
		</label>
		<label>
			Password
			<input type="password" bind:value={password} required autocomplete="current-password" />
		</label>
		{#if error}
			<p class="error">{error}</p>
		{/if}
		<button type="submit" disabled={submitting}>Sign in</button>
	</form>
</main>

<style>
	.login {
		display: flex;
		justify-content: center;
		align-items: center;
		min-height: 100vh;
	}
	form {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		width: 20rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.9rem;
	}
	.error {
		color: #c0392b;
		font-size: 0.85rem;
	}
</style>
