export interface ImportPreviewHost {
	label: string;
	hostname: string | null;
	os: string | null;
	addresses: string[];
	service_count: number;
	action: 'create' | 'merge';
}

export interface ImportPreview {
	hosts: ImportPreviewHost[];
	finding_count: number;
	trust_relationship_count: number;
}

export interface ImportResult {
	created_hosts: number;
	merged_hosts: number;
	services_added: number;
	findings_added: number;
	trust_relationships_added: number;
}

async function upload<T>(source: string, action: 'preview' | 'commit', engagementId: string, file: File): Promise<T> {
	const form = new FormData();
	form.append('engagement_id', engagementId);
	form.append('file', file);

	const res = await fetch(`/api/import/${source}/${action}`, {
		method: 'POST',
		credentials: 'same-origin',
		body: form
	});
	if (!res.ok) {
		throw new Error(`Import ${action} failed: ${res.status}`);
	}
	return res.json() as Promise<T>;
}

export function previewImport(source: string, engagementId: string, file: File): Promise<ImportPreview> {
	return upload<ImportPreview>(source, 'preview', engagementId, file);
}

export function commitImport(source: string, engagementId: string, file: File): Promise<ImportResult> {
	return upload<ImportResult>(source, 'commit', engagementId, file);
}
