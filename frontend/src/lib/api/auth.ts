export interface CurrentUser {
	id: string;
	email: string;
	display_name: string;
	is_admin: boolean;
}

async function handle<T>(res: Response): Promise<T> {
	if (!res.ok) {
		throw new Error(`Request failed: ${res.status}`);
	}
	return res.json() as Promise<T>;
}

export function login(email: string, password: string): Promise<CurrentUser> {
	return fetch('/api/auth/login', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		credentials: 'same-origin',
		body: JSON.stringify({ email, password })
	}).then((res) => handle<CurrentUser>(res));
}

export async function logout(): Promise<void> {
	await fetch('/api/auth/logout', { method: 'POST', credentials: 'same-origin' });
}

export function me(): Promise<CurrentUser> {
	return fetch('/api/auth/me', { credentials: 'same-origin' }).then((res) => handle<CurrentUser>(res));
}
