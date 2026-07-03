import { apiGet } from './http';

export interface SearchResult {
	result_type: string;
	id: string;
	title: string;
	snippet: string;
}

export function search(
	engagementId: string,
	query: string,
	types: string[] = []
): Promise<SearchResult[]> {
	const params = new URLSearchParams({ q: query });
	if (types.length > 0) params.set('types', types.join(','));
	return apiGet(`/api/engagements/${engagementId}/search?${params}`);
}
