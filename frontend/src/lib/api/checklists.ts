import { apiGet, apiSend } from './http';

export interface ChecklistItem {
	id: string;
	checklist_id: string;
	text: string;
	state: 'todo' | 'doing' | 'done' | 'na';
	assessment: 'safe' | 'undecided' | 'exploit';
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

export function listEngagementChecklists(engagementId: string): Promise<Checklist[]> {
	return apiGet(`/api/engagements/${engagementId}/checklists`);
}

/** Throws (404) if this service's type has no mapped checklist template, or none has been instantiated yet. */
export function getServiceChecklist(serviceId: string): Promise<Checklist> {
	return apiGet(`/api/services/${serviceId}/checklist`);
}

export function updateChecklistItem(
	itemId: string,
	state: string,
	assessment: string
): Promise<ChecklistItem> {
	return apiSend(`/api/checklist-items/${itemId}`, 'PUT', { state, assessment });
}
