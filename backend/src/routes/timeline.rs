use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct TimelineEvent {
    at: DateTime<Utc>,
    event_type: String,
    subject_type: String,
    subject_id: Uuid,
    title: String,
    summary: Option<String>,
}

#[derive(Deserialize)]
struct TimelineQuery {
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
}

async fn get_timeline(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineEvent>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let events = sqlx::query_as::<_, TimelineEvent>(
        "SELECT at, event_type, subject_type, subject_id, title, summary
         FROM timeline_events
         WHERE engagement_id = $1
           AND ($2::timestamptz IS NULL OR at >= $2)
           AND ($3::timestamptz IS NULL OR at <= $3)
         ORDER BY at ASC",
    )
    .bind(engagement_id)
    .bind(query.since)
    .bind(query.until)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(events))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/engagements/{engagement_id}/timeline", get(get_timeline))
}
