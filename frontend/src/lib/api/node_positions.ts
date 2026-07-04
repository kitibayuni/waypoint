import { apiGet, apiSend } from './http';

export interface NodePosition {
	node_id: string;
	x: number;
	y: number;
}

export function listNodePositions(engagementId: string): Promise<NodePosition[]> {
	return apiGet(`/api/engagements/${engagementId}/node-positions`);
}

export function putNodePositions(engagementId: string, positions: NodePosition[]): Promise<void> {
	return apiSend(`/api/engagements/${engagementId}/node-positions`, 'PUT', positions);
}
