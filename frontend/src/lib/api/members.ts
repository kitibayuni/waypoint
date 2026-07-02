import { apiDelete, apiGet, apiSend } from './http';

export interface Member {
	user_id: string;
	email: string;
	display_name: string;
	role: 'viewer' | 'tester' | 'lead';
	added_at: string;
}

export function listMembers(engagementId: string): Promise<Member[]> {
	return apiGet(`/api/engagements/${engagementId}/members`);
}

export function addMember(engagementId: string, email: string, role: string): Promise<Member> {
	return apiSend(`/api/engagements/${engagementId}/members`, 'POST', { email, role });
}

export function updateMemberRole(engagementId: string, userId: string, role: string): Promise<Member> {
	return apiSend(`/api/engagements/${engagementId}/members/${userId}`, 'PUT', { role });
}

export function removeMember(engagementId: string, userId: string): Promise<void> {
	return apiDelete(`/api/engagements/${engagementId}/members/${userId}`);
}
