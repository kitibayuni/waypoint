import { apiDelete, apiGet, apiSend } from './http';

export interface ObservationType {
	id: string;
	key: string;
	title: string;
	category: string;
	default_severity: string;
	description_md: string;
	references_json: unknown;
	mitre_technique_ids: unknown;
	created_at: string;
}

export interface Observation {
	id: string;
	host_id: string;
	service_id: string | null;
	observation_type_id: string;
	observation_key: string;
	observation_title: string;
	category: string;
	default_severity: string;
	severity_override: string | null;
	status: 'suspected' | 'confirmed' | 'remediated' | 'false_positive';
	evidence_md: string;
	created_by: string | null;
	created_at: string;
}

export interface CreateObservationRequest {
	observation_type_id: string;
	service_id?: string | null;
	status?: string;
	evidence_md?: string;
	severity_override?: string | null;
}

export interface UpdateObservationRequest {
	status: string;
	evidence_md: string;
	severity_override: string | null;
}

export function listObservationTypes(): Promise<ObservationType[]> {
	return apiGet('/api/observation-types');
}

export function listObservations(hostId: string): Promise<Observation[]> {
	return apiGet(`/api/hosts/${hostId}/observations`);
}

export function createObservation(
	hostId: string,
	payload: CreateObservationRequest
): Promise<Observation> {
	return apiSend(`/api/hosts/${hostId}/observations`, 'POST', payload);
}

export function updateObservation(
	id: string,
	payload: UpdateObservationRequest
): Promise<Observation> {
	return apiSend(`/api/observations/${id}`, 'PUT', payload);
}

export function deleteObservation(id: string): Promise<void> {
	return apiDelete(`/api/observations/${id}`);
}
