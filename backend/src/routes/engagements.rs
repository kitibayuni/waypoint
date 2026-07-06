use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::common::{OptionExt, ResultExt};
use crate::state::AppState;

const VALID_STATUSES: [&str; 4] = ["planning", "active", "reporting", "closed"];
const VALID_REPORT_TYPES: [&str; 4] =
    ["vuln_assessment", "penetration_test", "attestation", "post_remediation"];

fn valid_status(s: &str) -> bool {
    VALID_STATUSES.contains(&s)
}

fn valid_report_type(s: &str) -> bool {
    VALID_REPORT_TYPES.contains(&s)
}

fn default_status() -> String {
    "planning".to_string()
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Engagement {
    id: Uuid,
    client_id: Uuid,
    client_name: String,
    name: String,
    status: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    global_notes_md: String,
    report_type: String,
    severity_definitions_md: String,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateEngagementRequest {
    client_id: Uuid,
    name: String,
    #[serde(default = "default_status")]
    status: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    #[serde(default)]
    global_notes_md: String,
}

#[derive(Deserialize)]
pub struct UpdateEngagementRequest {
    name: String,
    status: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    global_notes_md: String,
    report_type: String,
    severity_definitions_md: String,
}

const ENGAGEMENT_SELECT: &str = "SELECT e.id, e.client_id, c.name AS client_name, e.name,
    e.status::text AS status, e.start_date, e.end_date, e.global_notes_md,
    e.report_type::text AS report_type, e.severity_definitions_md,
    e.created_by, e.created_at
    FROM engagements e JOIN clients c ON c.id = e.client_id";

async fn list_engagements(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<Vec<Engagement>>, StatusCode> {
    let engagements = if user.is_admin {
        sqlx::query_as::<_, Engagement>(&format!(
            "{ENGAGEMENT_SELECT} ORDER BY e.created_at DESC"
        ))
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query_as::<_, Engagement>(&format!(
            "{ENGAGEMENT_SELECT} JOIN engagement_members m ON m.engagement_id = e.id
             WHERE m.user_id = $1 ORDER BY e.created_at DESC"
        ))
        .bind(user.id)
        .fetch_all(&state.pool)
        .await
    }
    .internal()?;

    Ok(Json(engagements))
}

async fn create_engagement(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(payload): Json<CreateEngagementRequest>,
) -> Result<Json<Engagement>, StatusCode> {
    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .internal()?;

    let (new_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO engagements (client_id, name, status, start_date, end_date, global_notes_md, created_by)
         VALUES ($1, $2, $3::engagement_status, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(payload.client_id)
    .bind(&payload.name)
    .bind(&payload.status)
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(&payload.global_notes_md)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    .internal()?;

    sqlx::query(
        "INSERT INTO engagement_members (engagement_id, user_id, role) VALUES ($1, $2, 'lead')",
    )
    .bind(new_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await
    .internal()?;

    tx.commit()
        .await
        .internal()?;

    let engagement =
        sqlx::query_as::<_, Engagement>(&format!("{ENGAGEMENT_SELECT} WHERE e.id = $1"))
            .bind(new_id)
            .fetch_one(&state.pool)
            .await
            .internal()?;

    Ok(Json(engagement))
}

async fn get_engagement(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Engagement>, StatusCode> {
    require_role(&state.pool, &user, id, EngagementRole::Viewer).await?;

    let engagement =
        sqlx::query_as::<_, Engagement>(&format!("{ENGAGEMENT_SELECT} WHERE e.id = $1"))
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .internal()?
            .or_404()?;

    Ok(Json(engagement))
}

async fn update_engagement(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateEngagementRequest>,
) -> Result<Json<Engagement>, StatusCode> {
    require_role(&state.pool, &user, id, EngagementRole::Tester).await?;

    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !valid_report_type(&payload.report_type) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = sqlx::query(
        "UPDATE engagements SET name = $1, status = $2::engagement_status, start_date = $3,
         end_date = $4, global_notes_md = $5, report_type = $6::report_type,
         severity_definitions_md = $7 WHERE id = $8",
    )
    .bind(&payload.name)
    .bind(&payload.status)
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(&payload.global_notes_md)
    .bind(&payload.report_type)
    .bind(&payload.severity_definitions_md)
    .bind(id)
    .execute(&state.pool)
    .await
    .internal()?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let engagement =
        sqlx::query_as::<_, Engagement>(&format!("{ENGAGEMENT_SELECT} WHERE e.id = $1"))
            .bind(id)
            .fetch_one(&state.pool)
            .await
            .internal()?;

    Ok(Json(engagement))
}

async fn delete_engagement(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    require_role(&state.pool, &user, id, EngagementRole::Lead).await?;

    let result = sqlx::query("DELETE FROM engagements WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .internal()?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/engagements",
            get(list_engagements).post(create_engagement),
        )
        .route(
            "/engagements/{id}",
            get(get_engagement)
                .put(update_engagement)
                .delete(delete_engagement),
        )
}
