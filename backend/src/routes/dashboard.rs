use std::collections::HashMap;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

#[derive(Serialize)]
pub struct EngagementSummary {
    id: Uuid,
    name: String,
    status: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    days_elapsed: Option<i64>,
    days_remaining: Option<i64>,
}

#[derive(Serialize)]
pub struct CredentialStats {
    total: i64,
    validated: i64,
    reused: i64,
}

#[derive(Serialize)]
pub struct ChecklistStats {
    total: i64,
    done: i64,
    na: i64,
    todo: i64,
    doing: i64,
    completion_pct: f64,
}

#[derive(Serialize)]
pub struct Dashboard {
    engagement: EngagementSummary,
    hosts_by_status: HashMap<String, i64>,
    findings_by_severity: HashMap<String, i64>,
    checklist: ChecklistStats,
    credentials: CredentialStats,
    scope_count: i64,
}

async fn get_dashboard(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Dashboard>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    #[derive(sqlx::FromRow)]
    struct EngRow {
        id: Uuid,
        name: String,
        status: String,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    }
    let eng = sqlx::query_as::<_, EngRow>(
        "SELECT id, name, status::text AS status, start_date, end_date FROM engagements WHERE id = $1",
    )
    .bind(engagement_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let today = Utc::now().date_naive();
    let days_elapsed = eng.start_date.map(|d| (today - d).num_days());
    let days_remaining = eng.end_date.map(|d| (d - today).num_days());

    let engagement = EngagementSummary {
        id: eng.id,
        name: eng.name,
        status: eng.status,
        start_date: eng.start_date,
        end_date: eng.end_date,
        days_elapsed,
        days_remaining,
    };

    let host_rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT status::text, COUNT(*) FROM hosts WHERE engagement_id = $1 GROUP BY status")
            .bind(engagement_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hosts_by_status: HashMap<String, i64> = host_rows.into_iter().collect();

    let finding_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT COALESCE(severity, 'none'), COUNT(*) FROM findings WHERE engagement_id = $1
         GROUP BY COALESCE(severity, 'none')",
    )
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let findings_by_severity: HashMap<String, i64> = finding_rows.into_iter().collect();

    let checklist_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT ci.state::text, COUNT(*) FROM checklist_items ci
         JOIN checklists c ON c.id = ci.checklist_id
         LEFT JOIN hosts h ON h.id = c.host_id
         WHERE c.engagement_id = $1 OR h.engagement_id = $1
         GROUP BY ci.state",
    )
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let checklist_map: HashMap<String, i64> = checklist_rows.into_iter().collect();
    let done = *checklist_map.get("done").unwrap_or(&0);
    let na = *checklist_map.get("na").unwrap_or(&0);
    let todo = *checklist_map.get("todo").unwrap_or(&0);
    let doing = *checklist_map.get("doing").unwrap_or(&0);
    let total = done + na + todo + doing;
    let completion_pct = if total > 0 {
        ((done + na) as f64 / total as f64 * 1000.0).round() / 10.0
    } else {
        0.0
    };
    let checklist = ChecklistStats {
        total,
        done,
        na,
        todo,
        doing,
        completion_pct,
    };

    #[derive(sqlx::FromRow)]
    struct CredRow {
        total: i64,
        validated: i64,
        reused: i64,
    }
    let cred_row = sqlx::query_as::<_, CredRow>(
        "WITH reused_creds AS (
            SELECT credential_id FROM credential_usage WHERE result = 'works'
            GROUP BY credential_id HAVING COUNT(DISTINCT host_id) >= 2
         )
         SELECT COUNT(*) AS total,
                COUNT(*) FILTER (WHERE validated) AS validated,
                COUNT(*) FILTER (WHERE id IN (SELECT credential_id FROM reused_creds)) AS reused
         FROM credentials WHERE engagement_id = $1",
    )
    .bind(engagement_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let credentials = CredentialStats {
        total: cred_row.total,
        validated: cred_row.validated,
        reused: cred_row.reused,
    };

    let (scope_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM scope_items WHERE engagement_id = $1")
            .bind(engagement_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(Dashboard {
        engagement,
        hosts_by_status,
        findings_by_severity,
        checklist,
        credentials,
        scope_count,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/engagements/{engagement_id}/dashboard",
        axum::routing::get(get_dashboard),
    )
}
