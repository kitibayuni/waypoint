import { apiDelete, apiGet, apiSend } from './http';

export interface ServiceTechnology {
	id: string;
	service_id: string;
	name: string;
	version: string | null;
	notes_md: string;
	created_at: string;
}

export interface ServiceTechnologyRequest {
	name: string;
	version?: string | null;
	notes_md?: string;
}

// Free text is accepted for `name`; this is only a light suggestion list for
// the ones with a starter checklist template (see 0031_service_technologies.sql).
export const KNOWN_TECHNOLOGIES = ['wordpress', 'jenkins'];

export function listServiceTechnologies(serviceId: string): Promise<ServiceTechnology[]> {
	return apiGet(`/api/services/${serviceId}/technologies`);
}

export function createServiceTechnology(
	serviceId: string,
	payload: ServiceTechnologyRequest
): Promise<ServiceTechnology> {
	return apiSend(`/api/services/${serviceId}/technologies`, 'POST', payload);
}

export function deleteServiceTechnology(id: string): Promise<void> {
	return apiDelete(`/api/service-technologies/${id}`);
}
