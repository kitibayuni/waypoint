import { apiDelete, apiGet, apiSend } from './http';

export interface PivotTunnel {
	id: string;
	engagement_id: string;
	from_host_id: string;
	from_host_label: string;
	to_host_id: string | null;
	to_host_label: string | null;
	method: string;
	local_port: number | null;
	remote_target: string | null;
	notes_md: string;
	created_at: string;
	created_by_name: string | null;
}

export interface PivotTunnelRequest {
	from_host_id: string;
	to_host_id?: string | null;
	method: string;
	local_port?: number | null;
	remote_target?: string | null;
	notes_md?: string;
}

export const PIVOT_METHODS = [
	'ssh_dynamic',
	'ssh_local',
	'ssh_remote',
	'chisel',
	'ligolo',
	'sshuttle',
	'socat',
	'metasploit_autoroute',
	'dns_tunnel',
	'icmp_tunnel',
	'other'
];

export function listPivotTunnels(engagementId: string): Promise<PivotTunnel[]> {
	return apiGet(`/api/engagements/${engagementId}/pivot-tunnels`);
}

export function createPivotTunnel(
	engagementId: string,
	payload: PivotTunnelRequest
): Promise<PivotTunnel> {
	return apiSend(`/api/engagements/${engagementId}/pivot-tunnels`, 'POST', payload);
}

export function updatePivotTunnel(id: string, payload: PivotTunnelRequest): Promise<PivotTunnel> {
	return apiSend(`/api/pivot-tunnels/${id}`, 'PUT', payload);
}

export function deletePivotTunnel(id: string): Promise<void> {
	return apiDelete(`/api/pivot-tunnels/${id}`);
}
