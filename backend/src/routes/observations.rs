use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::hosts::host_engagement_id;
use crate::state::AppState;

const VALID_STATUSES: [&str; 4] = ["suspected", "confirmed", "remediated", "false_positive"];

fn valid_status(s: &str) -> bool {
    VALID_STATUSES.contains(&s)
}

fn default_status() -> String {
    "suspected".to_string()
}

async fn observation_engagement_id(pool: &PgPool, observation_id: Uuid) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>(
        "SELECT h.engagement_id FROM observations o JOIN hosts h ON h.id = o.host_id WHERE o.id = $1",
    )
    .bind(observation_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|(id,)| id)
    .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Observation {
    id: Uuid,
    host_id: Uuid,
    service_id: Option<Uuid>,
    observation_type_id: Uuid,
    observation_key: String,
    observation_title: String,
    category: String,
    default_severity: String,
    severity_override: Option<String>,
    status: String,
    evidence_md: String,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
}

const OBSERVATION_SELECT: &str = "SELECT o.id, o.host_id, o.service_id, o.observation_type_id,
    ot.key AS observation_key, ot.title AS observation_title, ot.category,
    ot.default_severity, o.severity_override, o.status::text AS status,
    o.evidence_md, o.created_by, o.created_at
    FROM observations o JOIN observation_types ot ON ot.id = o.observation_type_id";

#[derive(Deserialize)]
pub struct CreateObservationRequest {
    observation_type_id: Uuid,
    service_id: Option<Uuid>,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    evidence_md: String,
    severity_override: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateObservationRequest {
    status: String,
    evidence_md: String,
    severity_override: Option<String>,
}

async fn list_observations(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
) -> Result<Json<Vec<Observation>>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let observations = sqlx::query_as::<_, Observation>(&format!(
        "{OBSERVATION_SELECT} WHERE o.host_id = $1 ORDER BY o.created_at"
    ))
    .bind(host_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(observations))
}

async fn create_observation(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
    Json(payload): Json<CreateObservationRequest>,
) -> Result<Json<Observation>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (obs_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO observations (host_id, service_id, observation_type_id, severity_override, status, evidence_md, created_by)
         VALUES ($1, $2, $3, $4, $5::observation_status, $6, $7)
         RETURNING id",
    )
    .bind(host_id)
    .bind(payload.service_id)
    .bind(payload.observation_type_id)
    .bind(&payload.severity_override)
    .bind(&payload.status)
    .bind(&payload.evidence_md)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let observation =
        sqlx::query_as::<_, Observation>(&format!("{OBSERVATION_SELECT} WHERE o.id = $1"))
            .bind(obs_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(observation))
}

async fn update_observation(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateObservationRequest>,
) -> Result<Json<Observation>, StatusCode> {
    let engagement_id = observation_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = sqlx::query(
        "UPDATE observations SET status = $1::observation_status, evidence_md = $2, severity_override = $3
         WHERE id = $4",
    )
    .bind(&payload.status)
    .bind(&payload.evidence_md)
    .bind(&payload.severity_override)
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let observation =
        sqlx::query_as::<_, Observation>(&format!("{OBSERVATION_SELECT} WHERE o.id = $1"))
            .bind(id)
            .fetch_one(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(observation))
}

async fn delete_observation(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = observation_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM observations WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/hosts/{host_id}/observations",
            get(list_observations).post(create_observation),
        )
        .route(
            "/observations/{id}",
            axum::routing::put(update_observation).delete(delete_observation),
        )
}
