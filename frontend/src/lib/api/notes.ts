import { apiGet } from './http';

export interface Note {
	id: string;
	engagement_id: string;
	subject_type: string;
	subject_id: string;
	title: string | null;
	body_md: string;
	created_by: string | null;
	created_at: string;
}

export function listHostNotes(hostId: string): Promise<Note[]> {
	return apiGet(`/api/hosts/${hostId}/notes`);
}
