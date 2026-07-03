import { apiDelete, apiGet, apiSend } from './http';

export interface AffectedHost {
	id: string;
	label: string;
}

export interface Finding {
	id: string;
	engagement_id: string;
	title: string;
	cve: string | null;
	cvss_vector: string | null;
	cvss_score: number | null;
	severity: string | null;
	description_md: string;
	remediation_md: string;
	poc_md: string;
	references_json: unknown;
	status: 'open' | 'triaged' | 'accepted_risk' | 'fixed';
	source_observation_id: string | null;
	mitre_technique_ids: string[];
	created_at: string;
	affected_hosts: AffectedHost[];
}

export interface FindingRequest {
	title: string;
	cve?: string | null;
	cvss_vector?: string | null;
	cvss_score?: number | null;
	severity?: string | null;
	description_md?: string;
	remediation_md?: string;
	poc_md?: string;
	references_json?: unknown;
	status?: string;
	source_observation_id?: string | null;
	mitre_technique_ids?: string[];
	affected_host_ids?: string[];
}

export function listFindings(engagementId: string): Promise<Finding[]> {
	return apiGet(`/api/engagements/${engagementId}/findings`);
}

export function getFinding(id: string): Promise<Finding> {
	return apiGet(`/api/findings/${id}`);
}

export function createFinding(engagementId: string, payload: FindingRequest): Promise<Finding> {
	return apiSend(`/api/engagements/${engagementId}/findings`, 'POST', payload);
}

export function updateFinding(id: string, payload: FindingRequest): Promise<Finding> {
	return apiSend(`/api/findings/${id}`, 'PUT', payload);
}

export function deleteFinding(id: string): Promise<void> {
	return apiDelete(`/api/findings/${id}`);
}
