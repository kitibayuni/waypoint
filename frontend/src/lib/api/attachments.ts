import { apiDelete, apiGet } from './http';

export interface Attachment {
	id: string;
	engagement_id: string;
	subject_type: string;
	subject_id: string;
	filename: string;
	mime: string | null;
	size: number | null;
	sha256: string;
	caption: string | null;
	created_at: string;
}

export function listAttachments(
	engagementId: string,
	subjectType: string,
	subjectId: string
): Promise<Attachment[]> {
	const params = new URLSearchParams({
		engagement_id: engagementId,
		subject_type: subjectType,
		subject_id: subjectId
	});
	return apiGet(`/api/attachments?${params}`);
}

export async function uploadAttachment(
	engagementId: string,
	subjectType: string,
	subjectId: string,
	file: File,
	caption?: string
): Promise<Attachment> {
	const form = new FormData();
	form.append('engagement_id', engagementId);
	form.append('subject_type', subjectType);
	form.append('subject_id', subjectId);
	if (caption) form.append('caption', caption);
	form.append('file', file);

	const res = await fetch('/api/attachments', {
		method: 'POST',
		credentials: 'same-origin',
		body: form
	});
	if (!res.ok) {
		throw new Error(`Upload failed: ${res.status}`);
	}
	return res.json() as Promise<Attachment>;
}

export function downloadAttachmentUrl(id: string): string {
	return `/api/attachments/${id}/download`;
}

export function deleteAttachment(id: string): Promise<void> {
	return apiDelete(`/api/attachments/${id}`);
}
