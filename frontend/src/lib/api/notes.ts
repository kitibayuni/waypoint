import { apiDelete, apiGet, apiSend } from './http';

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

export interface CreateNoteRequest {
	engagement_id: string;
	subject_type: string;
	subject_id: string;
	title?: string | null;
	body_md?: string;
}

export interface UpdateNoteRequest {
	title: string | null;
	body_md: string;
}

export function listNotes(
	engagementId: string,
	subjectType: string,
	subjectId: string
): Promise<Note[]> {
	const params = new URLSearchParams({
		engagement_id: engagementId,
		subject_type: subjectType,
		subject_id: subjectId
	});
	return apiGet(`/api/notes?${params}`);
}

export function createNote(payload: CreateNoteRequest): Promise<Note> {
	return apiSend('/api/notes', 'POST', payload);
}

export function updateNote(id: string, payload: UpdateNoteRequest): Promise<Note> {
	return apiSend(`/api/notes/${id}`, 'PUT', payload);
}

export function deleteNote(id: string): Promise<void> {
	return apiDelete(`/api/notes/${id}`);
}
