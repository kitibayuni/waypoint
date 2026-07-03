import { apiGet } from './http';

export interface TimelineEvent {
	at: string;
	event_type: string;
	subject_type: string;
	subject_id: string;
	title: string;
	summary: string | null;
}

export function getTimeline(engagementId: string): Promise<TimelineEvent[]> {
	return apiGet(`/api/engagements/${engagementId}/timeline`);
}
