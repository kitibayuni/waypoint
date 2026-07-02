async function handle<T>(res: Response): Promise<T> {
	if (!res.ok) {
		throw new Error(`Request failed: ${res.status}`);
	}
	if (res.status === 204) {
		return undefined as T;
	}
	return res.json() as Promise<T>;
}

export function apiGet<T>(path: string): Promise<T> {
	return fetch(path, { credentials: 'same-origin' }).then((res) => handle<T>(res));
}

export function apiSend<T>(path: string, method: string, body: unknown): Promise<T> {
	return fetch(path, {
		method,
		credentials: 'same-origin',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	}).then((res) => handle<T>(res));
}

export function apiDelete(path: string): Promise<void> {
	return fetch(path, { method: 'DELETE', credentials: 'same-origin' }).then((res) => handle<void>(res));
}
