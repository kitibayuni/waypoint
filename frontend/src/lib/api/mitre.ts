import { apiGet } from './http';

export interface MitreTechnique {
	id: string;
	name: string;
	tactic: string | null;
	url: string | null;
}

export function listMitreTechniques(): Promise<MitreTechnique[]> {
	return apiGet('/api/mitre-techniques');
}
