use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::hosts::host_engagement_id;
use crate::state::AppState;

/// Read-only for now: lists notes attached to a host, just enough to view
/// what a template's note skeleton produced. Full notes CRUD (create,
/// update, delete, other subject types) is Phase 10's scope.
#[derive(Serialize, sqlx::FromRow)]
pub struct Note {
    id: Uuid,
    engagement_id: Uuid,
    subject_type: String,
    subject_id: Uuid,
    title: Option<String>,
    body_md: String,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
}

async fn list_host_notes(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
) -> Result<Json<Vec<Note>>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let notes = sqlx::query_as::<_, Note>(
        "SELECT id, engagement_id, subject_type::text AS subject_type, subject_id, title, body_md, created_by, created_at
         FROM notes WHERE subject_type = 'host' AND subject_id = $1 ORDER BY created_at",
    )
    .bind(host_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(notes))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/hosts/{host_id}/notes", get(list_host_notes))
}
