import { apiGet, apiSend } from './http';

export interface ReportSnapshot {
	id: string;
	engagement_id: string;
	generated_at: string;
	status: 'draft' | 'final';
	generated_by_name: string | null;
}

export const REPORT_TYPES = [
	{ value: 'penetration_test', label: 'Penetration Test' },
	{ value: 'vuln_assessment', label: 'Vulnerability Assessment' },
	{ value: 'attestation', label: 'Attestation' },
	{ value: 'post_remediation', label: 'Post-Remediation' }
];

export function reportViewUrl(engagementId: string): string {
	return `/api/reports/${engagementId}`;
}

export function reportPdfUrl(engagementId: string): string {
	return `/api/reports/${engagementId}?format=pdf`;
}

export function listSnapshots(engagementId: string): Promise<ReportSnapshot[]> {
	return apiGet(`/api/reports/${engagementId}/snapshots`);
}

export function createSnapshot(engagementId: string): Promise<ReportSnapshot> {
	return apiSend(`/api/reports/${engagementId}/snapshots`, 'POST', {});
}

export function markSnapshotFinal(
	engagementId: string,
	snapshotId: string
): Promise<ReportSnapshot> {
	return apiSend(`/api/reports/${engagementId}/snapshots/${snapshotId}/finalize`, 'PATCH', {});
}

export function snapshotViewUrl(engagementId: string, snapshotId: string): string {
	return `/api/reports/${engagementId}/snapshots/${snapshotId}`;
}

export function snapshotPdfUrl(engagementId: string, snapshotId: string): string {
	return `/api/reports/${engagementId}/snapshots/${snapshotId}?format=pdf`;
}
