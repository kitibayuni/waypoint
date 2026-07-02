import { apiGet } from './http';

export interface GraphNode {
	data: Record<string, unknown> & { id: string; type: string; label: string };
}

export interface GraphEdge {
	data: Record<string, unknown> & { id: string; source: string; target: string; type: string };
}

export interface GraphSuggestion {
	host_id: string;
	host_label: string;
	observation_key: string;
	observation_title: string;
	technique: string;
	outcome: string;
	next_step_md: string;
}

export interface Graph {
	nodes: GraphNode[];
	edges: GraphEdge[];
	suggestions: GraphSuggestion[];
}

export function getGraph(engagementId: string): Promise<Graph> {
	return apiGet(`/api/engagements/${engagementId}/graph`);
}
