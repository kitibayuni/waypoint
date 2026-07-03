import { apiDelete, apiGet, apiSend } from './http';

export const SERVICE_NAMES = [
	'ssh',
	'ftp',
	'telnet',
	'smb',
	'http',
	'https',
	'rdp',
	'winrm',
	'mssql',
	'mysql',
	'postgresql',
	'ldap',
	'dns',
	'snmp',
	'vnc',
	'other'
] as const;

export interface Service {
	id: string;
	host_id: string;
	port: number;
	protocol: 'tcp' | 'udp';
	name: string | null;
	display_name: string | null;
	version: string | null;
	banner: string | null;
	state: string | null;
	created_at: string;
}

export interface ServiceRequest {
	port: number;
	protocol?: string;
	name?: string | null;
	display_name?: string | null;
	version?: string | null;
	banner?: string | null;
	state?: string | null;
}

export function listServices(hostId: string): Promise<Service[]> {
	return apiGet(`/api/hosts/${hostId}/services`);
}

export function createService(hostId: string, payload: ServiceRequest): Promise<Service> {
	return apiSend(`/api/hosts/${hostId}/services`, 'POST', payload);
}

export function updateService(hostId: string, serviceId: string, payload: ServiceRequest): Promise<Service> {
	return apiSend(`/api/hosts/${hostId}/services/${serviceId}`, 'PUT', payload);
}

export function deleteService(hostId: string, serviceId: string): Promise<void> {
	return apiDelete(`/api/hosts/${hostId}/services/${serviceId}`);
}
