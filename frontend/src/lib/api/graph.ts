import { apiGet } from './http';

export interface GraphNode {
	data: Record<string, unknown> & { id: string; type: string; label: string };
}

export interface GraphEdge {
	data: Record<string, unknown> & { id: string; source: string; target: string; type: string };
}

export interface Graph {
	nodes: GraphNode[];
	edges: GraphEdge[];
}

export function getGraph(engagementId: string, asOf?: string): Promise<Graph> {
	const query = asOf ? `?as_of=${encodeURIComponent(asOf)}` : '';
	return apiGet(`/api/engagements/${engagementId}/graph${query}`);
}
