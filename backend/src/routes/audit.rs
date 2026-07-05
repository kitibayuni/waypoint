use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::routes::common::ResultExt;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct AuditEntry {
    id: Uuid,
    actor_email: Option<String>,
    action: String,
    subject_type: String,
    subject_id: Uuid,
    before: Option<Value>,
    after: Option<Value>,
    at: DateTime<Utc>,
}

/// Admin-only, system-wide (not engagement-scoped) -- audit_log spans every
/// engagement, and admins already see everything, so there's no per-
/// engagement RBAC to apply here.
async fn list_audit_log(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<Vec<AuditEntry>>, StatusCode> {
    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let entries = sqlx::query_as::<_, AuditEntry>(
        "SELECT a.id, u.email AS actor_email, a.action, a.subject_type, a.subject_id, a.before, a.after, a.at
         FROM audit_log a
         LEFT JOIN users u ON u.id = a.actor_id
         ORDER BY a.at DESC
         LIMIT 200",
    )
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(entries))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/audit-log", get(list_audit_log))
}
