import { apiDelete, apiGet, apiSend } from './http';

export interface ScopeItem {
	id: string;
	engagement_id: string;
	kind: 'ip' | 'cidr' | 'domain' | 'url' | 'asn' | 'exclusion';
	value: string;
	in_scope: boolean;
	note: string | null;
	created_at: string;
}

export interface ScopeItemRequest {
	kind: string;
	value: string;
	in_scope?: boolean;
	note?: string | null;
}

export function listScope(engagementId: string): Promise<ScopeItem[]> {
	return apiGet(`/api/engagements/${engagementId}/scope`);
}

export function createScopeItem(engagementId: string, payload: ScopeItemRequest): Promise<ScopeItem> {
	return apiSend(`/api/engagements/${engagementId}/scope`, 'POST', payload);
}

export function updateScopeItem(
	engagementId: string,
	scopeId: string,
	payload: ScopeItemRequest
): Promise<ScopeItem> {
	return apiSend(`/api/engagements/${engagementId}/scope/${scopeId}`, 'PUT', payload);
}

export function deleteScopeItem(engagementId: string, scopeId: string): Promise<void> {
	return apiDelete(`/api/engagements/${engagementId}/scope/${scopeId}`);
}
