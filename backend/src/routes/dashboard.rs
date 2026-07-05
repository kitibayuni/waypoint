use std::collections::HashMap;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::common::{OptionExt, ResultExt};
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

async fn fetch_engagement_summary(
    pool: &PgPool,
    engagement_id: Uuid,
) -> Result<Option<EngagementSummary>, StatusCode> {
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
    .fetch_optional(pool)
    .await
    .internal()?;

    let Some(eng) = eng else {
        return Ok(None);
    };

    let today = Utc::now().date_naive();
    Ok(Some(EngagementSummary {
        id: eng.id,
        name: eng.name,
        status: eng.status,
        days_elapsed: eng.start_date.map(|d| (today - d).num_days()),
        days_remaining: eng.end_date.map(|d| (d - today).num_days()),
        start_date: eng.start_date,
        end_date: eng.end_date,
    }))
}

async fn fetch_hosts_by_status(
    pool: &PgPool,
    engagement_id: Uuid,
) -> Result<HashMap<String, i64>, StatusCode> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT status::text, COUNT(*) FROM hosts WHERE engagement_id = $1 GROUP BY status")
            .bind(engagement_id)
            .fetch_all(pool)
            .await
            .internal()?;
    Ok(rows.into_iter().collect())
}

async fn fetch_findings_by_severity(
    pool: &PgPool,
    engagement_id: Uuid,
) -> Result<HashMap<String, i64>, StatusCode> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT COALESCE(severity, 'none'), COUNT(*) FROM findings WHERE engagement_id = $1
         GROUP BY COALESCE(severity, 'none')",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await
    .internal()?;
    Ok(rows.into_iter().collect())
}

async fn fetch_checklist_stats(pool: &PgPool, engagement_id: Uuid) -> Result<ChecklistStats, StatusCode> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT ci.state::text, COUNT(*) FROM checklist_items ci
         JOIN checklists c ON c.id = ci.checklist_id
         LEFT JOIN hosts h ON h.id = c.host_id
         WHERE c.engagement_id = $1 OR h.engagement_id = $1
         GROUP BY ci.state",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await
    .internal()?;

    let by_state: HashMap<String, i64> = rows.into_iter().collect();
    let done = *by_state.get("done").unwrap_or(&0);
    let na = *by_state.get("na").unwrap_or(&0);
    let todo = *by_state.get("todo").unwrap_or(&0);
    let doing = *by_state.get("doing").unwrap_or(&0);
    let total = done + na + todo + doing;
    let completion_pct = if total > 0 {
        ((done + na) as f64 / total as f64 * 1000.0).round() / 10.0
    } else {
        0.0
    };

    Ok(ChecklistStats {
        total,
        done,
        na,
        todo,
        doing,
        completion_pct,
    })
}

async fn fetch_credential_stats(pool: &PgPool, engagement_id: Uuid) -> Result<CredentialStats, StatusCode> {
    #[derive(sqlx::FromRow)]
    struct CredRow {
        total: i64,
        validated: i64,
        reused: i64,
    }
    let row = sqlx::query_as::<_, CredRow>(
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
    .fetch_one(pool)
    .await
    .internal()?;

    Ok(CredentialStats {
        total: row.total,
        validated: row.validated,
        reused: row.reused,
    })
}

async fn fetch_scope_count(pool: &PgPool, engagement_id: Uuid) -> Result<i64, StatusCode> {
    let (scope_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scope_items WHERE engagement_id = $1")
        .bind(engagement_id)
        .fetch_one(pool)
        .await
        .internal()?;
    Ok(scope_count)
}

async fn get_dashboard(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Dashboard>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    // Everything below is an independent read against the same engagement_id --
    // running them concurrently rather than one after another means the
    // handler's latency is the slowest single query, not the sum of all six.
    let (engagement, hosts_by_status, findings_by_severity, checklist, credentials, scope_count) = tokio::try_join!(
        fetch_engagement_summary(&state.pool, engagement_id),
        fetch_hosts_by_status(&state.pool, engagement_id),
        fetch_findings_by_severity(&state.pool, engagement_id),
        fetch_checklist_stats(&state.pool, engagement_id),
        fetch_credential_stats(&state.pool, engagement_id),
        fetch_scope_count(&state.pool, engagement_id),
    )?;
    let engagement = engagement.or_404()?;

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
