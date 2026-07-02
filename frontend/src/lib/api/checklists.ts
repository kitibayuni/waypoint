import { apiGet, apiSend } from './http';

export interface ChecklistItem {
	id: string;
	checklist_id: string;
	text: string;
	state: 'todo' | 'doing' | 'done' | 'na';
	position: number;
}

export interface Checklist {
	id: string;
	host_id: string | null;
	engagement_id: string | null;
	name: string;
	items: ChecklistItem[];
}

export function listHostChecklists(hostId: string): Promise<Checklist[]> {
	return apiGet(`/api/hosts/${hostId}/checklists`);
}

export function updateChecklistItemState(itemId: string, state: string): Promise<ChecklistItem> {
	return apiSend(`/api/checklist-items/${itemId}`, 'PUT', { state });
}
