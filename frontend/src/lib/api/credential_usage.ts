import { apiDelete, apiGet, apiSend } from './http';

export interface CredentialUsage {
	id: string;
	credential_id: string;
	host_id: string;
	host_label: string;
	service_id: string | null;
	result: 'works' | 'fails' | 'untested';
	privilege: 'user' | 'admin' | 'domain_admin' | 'system' | null;
	created_at: string;
}

export interface CreateUsageRequest {
	host_id: string;
	service_id?: string | null;
	result?: string;
	privilege?: string | null;
}

export interface UpdateUsageRequest {
	result: string;
	privilege?: string | null;
}

export function listUsage(credentialId: string): Promise<CredentialUsage[]> {
	return apiGet(`/api/credentials/${credentialId}/usage`);
}

export function createUsage(credentialId: string, payload: CreateUsageRequest): Promise<CredentialUsage> {
	return apiSend(`/api/credentials/${credentialId}/usage`, 'POST', payload);
}

export function updateUsage(id: string, payload: UpdateUsageRequest): Promise<CredentialUsage> {
	return apiSend(`/api/credential-usage/${id}`, 'PUT', payload);
}

export function deleteUsage(id: string): Promise<void> {
	return apiDelete(`/api/credential-usage/${id}`);
}
