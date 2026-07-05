use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::log_action;
use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::common::{scoped_engagement_id, OptionExt, ResultExt};
use crate::state::AppState;

const VALID_STATUSES: [&str; 4] = ["open", "triaged", "accepted_risk", "fixed"];

fn valid_status(s: &str) -> bool {
    VALID_STATUSES.contains(&s)
}

fn default_status() -> String {
    "open".to_string()
}

pub(crate) async fn finding_engagement_id(pool: &PgPool, id: Uuid) -> Result<Uuid, StatusCode> {
    scoped_engagement_id(pool, "SELECT engagement_id FROM findings WHERE id = $1", id).await
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Finding {
    id: Uuid,
    engagement_id: Uuid,
    title: String,
    cve: Option<String>,
    cvss_vector: Option<String>,
    cvss_score: Option<f64>,
    severity: Option<String>,
    description_md: String,
    remediation_md: String,
    poc_md: String,
    references_json: Value,
    status: String,
    mitre_technique_ids: Value,
    created_at: DateTime<Utc>,
    #[sqlx(json)]
    affected_hosts: Vec<AffectedHost>,
}

#[derive(Serialize, Deserialize)]
pub struct AffectedHost {
    id: Uuid,
    label: String,
}

const FINDING_SELECT: &str = "SELECT f.id, f.engagement_id, f.title, f.cve, f.cvss_vector,
    f.cvss_score::float8 AS cvss_score, f.severity, f.description_md, f.remediation_md, f.poc_md,
    f.references_json, f.status::text AS status, f.mitre_technique_ids, f.created_at,
    COALESCE(
        jsonb_agg(DISTINCT jsonb_build_object('id', h.id, 'label', h.label)) FILTER (WHERE h.id IS NOT NULL),
        '[]'
    ) AS affected_hosts
    FROM findings f
    LEFT JOIN finding_hosts fh ON fh.finding_id = f.id
    LEFT JOIN hosts h ON h.id = fh.host_id";

#[derive(Deserialize)]
pub struct FindingRequest {
    title: String,
    cve: Option<String>,
    cvss_vector: Option<String>,
    cvss_score: Option<f64>,
    severity: Option<String>,
    #[serde(default)]
    description_md: String,
    #[serde(default)]
    remediation_md: String,
    #[serde(default)]
    poc_md: String,
    #[serde(default)]
    references_json: Value,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    mitre_technique_ids: Value,
    #[serde(default)]
    affected_host_ids: Vec<Uuid>,
}

fn normalize_refs(v: Value) -> Value {
    if v.is_null() {
        serde_json::json!([])
    } else {
        v
    }
}

async fn list_findings(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<Finding>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let findings = sqlx::query_as::<_, Finding>(&format!(
        "{FINDING_SELECT} WHERE f.engagement_id = $1 GROUP BY f.id ORDER BY f.created_at DESC"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(findings))
}

async fn create_finding(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<FindingRequest>,
) -> Result<Json<Finding>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .internal()?;

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO findings (engagement_id, title, cve, cvss_vector, cvss_score, severity,
         description_md, remediation_md, poc_md, references_json, status, mitre_technique_ids)
         VALUES ($1, $2, $3, $4, $5::numeric, $6, $7, $8, $9, $10, $11::finding_status, $12)
         RETURNING id",
    )
    .bind(engagement_id)
    .bind(&payload.title)
    .bind(&payload.cve)
    .bind(&payload.cvss_vector)
    .bind(payload.cvss_score)
    .bind(&payload.severity)
    .bind(&payload.description_md)
    .bind(&payload.remediation_md)
    .bind(&payload.poc_md)
    .bind(normalize_refs(payload.references_json.clone()))
    .bind(&payload.status)
    .bind(normalize_refs(payload.mitre_technique_ids.clone()))
    .fetch_one(&mut *tx)
    .await
    .internal()?;

    for host_id in &payload.affected_host_ids {
        sqlx::query("INSERT INTO finding_hosts (finding_id, host_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(id)
            .bind(host_id)
            .execute(&mut *tx)
            .await
            .internal()?;
    }

    tx.commit()
        .await
        .internal()?;

    let finding = sqlx::query_as::<_, Finding>(&format!("{FINDING_SELECT} WHERE f.id = $1 GROUP BY f.id"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    log_action(&state.pool, user.id, "create", "finding", id, None::<&Finding>, Some(&finding)).await;

    Ok(Json(finding))
}

async fn get_finding(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Finding>, StatusCode> {
    let engagement_id = finding_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let finding = sqlx::query_as::<_, Finding>(&format!("{FINDING_SELECT} WHERE f.id = $1 GROUP BY f.id"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .internal()?
        .or_404()?;

    Ok(Json(finding))
}

async fn update_finding(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<FindingRequest>,
) -> Result<Json<Finding>, StatusCode> {
    let engagement_id = finding_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let before = sqlx::query_as::<_, Finding>(&format!("{FINDING_SELECT} WHERE f.id = $1 GROUP BY f.id"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .internal()?
        .or_404()?;

    let mut tx = state
        .pool
        .begin()
        .await
        .internal()?;

    let result = sqlx::query(
        "UPDATE findings SET title = $1, cve = $2, cvss_vector = $3, cvss_score = $4::numeric,
         severity = $5, description_md = $6, remediation_md = $7, poc_md = $8, references_json = $9,
         status = $10::finding_status, mitre_technique_ids = $11 WHERE id = $12",
    )
    .bind(&payload.title)
    .bind(&payload.cve)
    .bind(&payload.cvss_vector)
    .bind(payload.cvss_score)
    .bind(&payload.severity)
    .bind(&payload.description_md)
    .bind(&payload.remediation_md)
    .bind(&payload.poc_md)
    .bind(normalize_refs(payload.references_json.clone()))
    .bind(&payload.status)
    .bind(normalize_refs(payload.mitre_technique_ids.clone()))
    .bind(id)
    .execute(&mut *tx)
    .await
    .internal()?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    sqlx::query("DELETE FROM finding_hosts WHERE finding_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .internal()?;

    for host_id in &payload.affected_host_ids {
        sqlx::query("INSERT INTO finding_hosts (finding_id, host_id) VALUES ($1, $2)")
            .bind(id)
            .bind(host_id)
            .execute(&mut *tx)
            .await
            .internal()?;
    }

    tx.commit()
        .await
        .internal()?;

    let finding = sqlx::query_as::<_, Finding>(&format!("{FINDING_SELECT} WHERE f.id = $1 GROUP BY f.id"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    log_action(&state.pool, user.id, "update", "finding", id, Some(&before), Some(&finding)).await;

    Ok(Json(finding))
}

async fn delete_finding(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = finding_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let before = sqlx::query_as::<_, Finding>(&format!("{FINDING_SELECT} WHERE f.id = $1 GROUP BY f.id"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .internal()?
        .or_404()?;

    let result = sqlx::query("DELETE FROM findings WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .internal()?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    log_action(&state.pool, user.id, "delete", "finding", id, Some(&before), None::<&Finding>).await;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct HistoryEntry {
    id: Uuid,
    action: String,
    actor_email: Option<String>,
    before: Option<Value>,
    after: Option<Value>,
    at: DateTime<Utc>,
}

/// Version history for a finding, reusing audit_log rather than a separate
/// history table -- each audit entry's before/after snapshot already IS a
/// version diff; a dedicated row-versioning table would just duplicate it.
async fn get_finding_history(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<HistoryEntry>>, StatusCode> {
    let engagement_id = finding_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let history = sqlx::query_as::<_, HistoryEntry>(
        "SELECT a.id, a.action, u.email AS actor_email, a.before, a.after, a.at
         FROM audit_log a
         LEFT JOIN users u ON u.id = a.actor_id
         WHERE a.subject_type = 'finding' AND a.subject_id = $1
         ORDER BY a.at DESC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(history))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/engagements/{engagement_id}/findings",
            get(list_findings).post(create_finding),
        )
        .route(
            "/findings/{id}",
            get(get_finding).put(update_finding).delete(delete_finding),
        )
        .route("/findings/{id}/history", get(get_finding_history))
}
