import { apiDelete, apiGet, apiSend } from './http';

export interface Engagement {
	id: string;
	client_id: string;
	client_name: string;
	name: string;
	status: 'planning' | 'active' | 'reporting' | 'closed';
	start_date: string | null;
	end_date: string | null;
	global_notes_md: string;
	created_by: string | null;
	created_at: string;
}

export interface CreateEngagementRequest {
	client_id: string;
	name: string;
	status?: string;
	start_date?: string | null;
	end_date?: string | null;
	global_notes_md?: string;
}

export interface UpdateEngagementRequest {
	name: string;
	status: string;
	start_date: string | null;
	end_date: string | null;
	global_notes_md: string;
}

export function listEngagements(): Promise<Engagement[]> {
	return apiGet('/api/engagements');
}

export function getEngagement(id: string): Promise<Engagement> {
	return apiGet(`/api/engagements/${id}`);
}

export function createEngagement(payload: CreateEngagementRequest): Promise<Engagement> {
	return apiSend('/api/engagements', 'POST', payload);
}

export function updateEngagement(id: string, payload: UpdateEngagementRequest): Promise<Engagement> {
	return apiSend(`/api/engagements/${id}`, 'PUT', payload);
}

export function deleteEngagement(id: string): Promise<void> {
	return apiDelete(`/api/engagements/${id}`);
}
