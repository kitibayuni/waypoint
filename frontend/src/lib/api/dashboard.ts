import { apiGet } from './http';

export interface EngagementSummary {
	id: string;
	name: string;
	status: string;
	start_date: string | null;
	end_date: string | null;
	days_elapsed: number | null;
	days_remaining: number | null;
}

export interface ChecklistStats {
	total: number;
	done: number;
	na: number;
	todo: number;
	doing: number;
	completion_pct: number;
}

export interface CredentialStats {
	total: number;
	validated: number;
	reused: number;
}

export interface Dashboard {
	engagement: EngagementSummary;
	hosts_by_status: Record<string, number>;
	observations_by_status: Record<string, number>;
	findings_by_severity: Record<string, number>;
	checklist: ChecklistStats;
	credentials: CredentialStats;
	scope_count: number;
}

export function getDashboard(engagementId: string): Promise<Dashboard> {
	return apiGet(`/api/engagements/${engagementId}/dashboard`);
}
