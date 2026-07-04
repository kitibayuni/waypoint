import { apiDelete, apiGet, apiSend } from './http';

export interface HostTag {
	id: string;
	name: string;
}

export interface HostAddress {
	id: string;
	ip: string;
	is_primary: boolean;
}

export interface Host {
	id: string;
	engagement_id: string;
	label: string;
	hostname: string | null;
	os: string | null;
	os_family: string | null;
	criticality: string | null;
	status: 'discovered' | 'enumerating' | 'exploited' | 'owned' | 'cleared';
	general_info_md: string;
	login_notes_md: string;
	is_foothold: boolean;
	is_pivot: boolean;
	created_at: string;
	addresses: HostAddress[];
	tags: HostTag[];
}

export interface CreateHostRequest {
	label: string;
	hostname?: string | null;
	os?: string | null;
	os_family?: string | null;
	criticality?: string | null;
	status?: string;
	general_info_md?: string;
	addresses?: string[];
	tags?: string[];
}

export interface UpdateHostRequest {
	label: string;
	hostname: string | null;
	os: string | null;
	os_family: string | null;
	criticality: string | null;
	status: string;
	general_info_md: string;
	login_notes_md: string;
	is_foothold: boolean;
	is_pivot: boolean;
}

export interface Tag {
	id: string;
	name: string;
}

export function listHosts(engagementId: string): Promise<Host[]> {
	return apiGet(`/api/engagements/${engagementId}/hosts`);
}

export function getHost(id: string): Promise<Host> {
	return apiGet(`/api/hosts/${id}`);
}

export function createHost(engagementId: string, payload: CreateHostRequest): Promise<Host> {
	return apiSend(`/api/engagements/${engagementId}/hosts`, 'POST', payload);
}

export function updateHost(id: string, payload: UpdateHostRequest): Promise<Host> {
	return apiSend(`/api/hosts/${id}`, 'PUT', payload);
}

export function deleteHost(id: string): Promise<void> {
	return apiDelete(`/api/hosts/${id}`);
}

export function addAddress(hostId: string, ip: string, isPrimary = false): Promise<Host> {
	return apiSend(`/api/hosts/${hostId}/addresses`, 'POST', { ip, is_primary: isPrimary });
}

export function removeAddress(hostId: string, addressId: string): Promise<void> {
	return apiDelete(`/api/hosts/${hostId}/addresses/${addressId}`);
}

export function addTag(hostId: string, name: string): Promise<Host> {
	return apiSend(`/api/hosts/${hostId}/tags`, 'POST', { name });
}

export function removeTag(hostId: string, tagId: string): Promise<void> {
	return apiDelete(`/api/hosts/${hostId}/tags/${tagId}`);
}

export function listEngagementTags(engagementId: string): Promise<Tag[]> {
	return apiGet(`/api/engagements/${engagementId}/tags`);
}
