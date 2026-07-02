import { apiDelete, apiGet, apiSend } from './http';

export interface Client {
	id: string;
	name: string;
	contacts: unknown;
	notes: string | null;
	created_at: string;
}

export interface ClientRequest {
	name: string;
	contacts?: unknown;
	notes?: string | null;
}

export function listClients(): Promise<Client[]> {
	return apiGet('/api/clients');
}

export function getClient(id: string): Promise<Client> {
	return apiGet(`/api/clients/${id}`);
}

export function createClient(payload: ClientRequest): Promise<Client> {
	return apiSend('/api/clients', 'POST', payload);
}

export function updateClient(id: string, payload: ClientRequest): Promise<Client> {
	return apiSend(`/api/clients/${id}`, 'PUT', payload);
}

export function deleteClient(id: string): Promise<void> {
	return apiDelete(`/api/clients/${id}`);
}
