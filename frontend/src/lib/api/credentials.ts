import { apiDelete, apiGet, apiSend } from './http';

export interface Credential {
	id: string;
	engagement_id: string;
	username: string;
	domain: string | null;
	secret_type: 'plaintext' | 'ntlm' | 'kerb' | 'ssh_key' | 'hash_other';
	source_host_id: string | null;
	origin: 'captured' | 'cracked' | 'sprayed' | 'default' | 'created';
	validated: boolean;
	notes_md: string;
	created_at: string;
}

export interface CreateCredentialRequest {
	username: string;
	domain?: string | null;
	secret: string;
	secret_type: string;
	source_host_id?: string | null;
	origin?: string;
	validated?: boolean;
	notes_md?: string;
}

export interface UpdateCredentialRequest {
	username: string;
	domain: string | null;
	secret?: string | null;
	secret_type: string;
	source_host_id: string | null;
	origin: string;
	validated: boolean;
	notes_md: string;
}

export function listCredentials(engagementId: string): Promise<Credential[]> {
	return apiGet(`/api/engagements/${engagementId}/credentials`);
}

export function createCredential(
	engagementId: string,
	payload: CreateCredentialRequest
): Promise<Credential> {
	return apiSend(`/api/engagements/${engagementId}/credentials`, 'POST', payload);
}

export function updateCredential(id: string, payload: UpdateCredentialRequest): Promise<Credential> {
	return apiSend(`/api/credentials/${id}`, 'PUT', payload);
}

export function deleteCredential(id: string): Promise<void> {
	return apiDelete(`/api/credentials/${id}`);
}

export function revealCredential(id: string): Promise<{ secret: string }> {
	return apiGet(`/api/credentials/${id}/reveal`);
}
