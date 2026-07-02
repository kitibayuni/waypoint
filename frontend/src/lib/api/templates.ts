import { apiDelete, apiGet, apiSend } from './http';

export interface Template {
	id: string;
	kind: 'host' | 'checklist' | 'finding' | 'note' | 'engagement';
	name: string;
	description: string | null;
	owner_id: string | null;
	is_shared: boolean;
	created_at: string;
	body: unknown;
}

export interface TemplateRequest {
	kind: string;
	name: string;
	description?: string | null;
	is_shared?: boolean;
	body?: unknown;
}

export interface InstantiateRequest {
	engagement_id?: string;
	host_id?: string;
	client_id?: string;
	name?: string;
	hostname?: string;
	os?: string;
	subject_type?: string;
	subject_id?: string;
}

export interface InstantiateResponse {
	kind: string;
	id: string;
	engagement_id: string;
}

export function listTemplates(): Promise<Template[]> {
	return apiGet('/api/templates');
}

export function createTemplate(payload: TemplateRequest): Promise<Template> {
	return apiSend('/api/templates', 'POST', payload);
}

export function updateTemplate(id: string, payload: TemplateRequest): Promise<Template> {
	return apiSend(`/api/templates/${id}`, 'PUT', payload);
}

export function deleteTemplate(id: string): Promise<void> {
	return apiDelete(`/api/templates/${id}`);
}

export function instantiateTemplate(
	id: string,
	payload: InstantiateRequest
): Promise<InstantiateResponse> {
	return apiSend(`/api/templates/${id}/instantiate`, 'POST', payload);
}
