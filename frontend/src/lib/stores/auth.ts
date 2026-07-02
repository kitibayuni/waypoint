import { writable } from 'svelte/store';
import * as api from '$lib/api/auth';
import type { CurrentUser } from '$lib/api/auth';

export const currentUser = writable<CurrentUser | null>(null);
export const authChecked = writable(false);

export async function login(email: string, password: string): Promise<CurrentUser> {
	const user = await api.login(email, password);
	currentUser.set(user);
	authChecked.set(true);
	return user;
}

export async function logout(): Promise<void> {
	await api.logout();
	currentUser.set(null);
}

export async function refreshMe(): Promise<void> {
	try {
		const user = await api.me();
		currentUser.set(user);
	} catch {
		currentUser.set(null);
	} finally {
		authChecked.set(true);
	}
}
