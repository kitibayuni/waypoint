import { apiGet } from './http';

export interface AuditEntry {
	id: string;
	actor_email: string | null;
	action: string;
	subject_type: string;
	subject_id: string;
	before: Record<string, unknown> | null;
	after: Record<string, unknown> | null;
	at: string;
}

export function listAuditLog(): Promise<AuditEntry[]> {
	return apiGet('/api/audit-log');
}

export interface FindingHistoryEntry {
	id: string;
	action: string;
	actor_email: string | null;
	before: Record<string, unknown> | null;
	after: Record<string, unknown> | null;
	at: string;
}

export function getFindingHistory(findingId: string): Promise<FindingHistoryEntry[]> {
	return apiGet(`/api/findings/${findingId}/history`);
}
