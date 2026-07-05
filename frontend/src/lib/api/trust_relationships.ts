import { apiDelete, apiGet, apiSend } from './http';

export interface TrustRelationship {
	id: string;
	engagement_id: string;
	from_host_id: string;
	from_host_label: string;
	to_host_id: string;
	to_host_label: string;
	kind: 'domain_trust' | 'admin_of' | 'shares_creds' | 'session';
	direction: string | null;
	note: string | null;
	created_at: string;
}

export interface TrustRelationshipRequest {
	from_host_id: string;
	to_host_id: string;
	kind: string;
	direction?: string | null;
	note?: string | null;
}

export function listTrustRelationships(engagementId: string): Promise<TrustRelationship[]> {
	return apiGet(`/api/engagements/${engagementId}/trust-relationships`);
}

export function createTrustRelationship(
	engagementId: string,
	payload: TrustRelationshipRequest
): Promise<TrustRelationship> {
	return apiSend(`/api/engagements/${engagementId}/trust-relationships`, 'POST', payload);
}

export function updateTrustRelationship(
	id: string,
	payload: TrustRelationshipRequest
): Promise<TrustRelationship> {
	return apiSend(`/api/trust-relationships/${id}`, 'PUT', payload);
}

export function deleteTrustRelationship(id: string): Promise<void> {
	return apiDelete(`/api/trust-relationships/${id}`);
}
