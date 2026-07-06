use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::common::{instantiate_checklist_if_mapped, OptionExt, ResultExt};
use crate::routes::hosts::host_engagement_id;
use crate::state::AppState;

/// A detected application/technology layered on top of a protocol-level
/// service (e.g. an `http` service that's specifically WordPress) -- see
/// `routes::services::maybe_auto_checklist` for the protocol-level
/// equivalent this mirrors.
#[derive(Serialize, sqlx::FromRow)]
pub struct ServiceTechnology {
    id: Uuid,
    service_id: Uuid,
    name: String,
    version: Option<String>,
    notes_md: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ServiceTechnologyRequest {
    name: String,
    version: Option<String>,
    #[serde(default)]
    notes_md: String,
}

const TECHNOLOGY_SELECT: &str =
    "SELECT id, service_id, name, version, notes_md, created_at FROM service_technologies";

async fn service_host_id(pool: &sqlx::PgPool, service_id: Uuid) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>("SELECT host_id FROM services WHERE id = $1")
        .bind(service_id)
        .fetch_optional(pool)
        .await
        .internal()?
        .map(|(id,)| id)
        .or_404()
}

async fn list_technologies(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<Vec<ServiceTechnology>>, StatusCode> {
    let host_id = service_host_id(&state.pool, service_id).await?;
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let technologies = sqlx::query_as::<_, ServiceTechnology>(&format!(
        "{TECHNOLOGY_SELECT} WHERE service_id = $1 ORDER BY created_at"
    ))
    .bind(service_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(technologies))
}

async fn create_technology(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(service_id): Path<Uuid>,
    Json(payload): Json<ServiceTechnologyRequest>,
) -> Result<Json<ServiceTechnology>, StatusCode> {
    let host_id = service_host_id(&state.pool, service_id).await?;
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let mut tx = state.pool.begin().await.internal()?;

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO service_technologies (service_id, name, version, notes_md)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(service_id)
    .bind(&payload.name)
    .bind(&payload.version)
    .bind(&payload.notes_md)
    .fetch_one(&mut *tx)
    .await
    .internal()?;

    instantiate_checklist_if_mapped(
        &mut tx,
        host_id,
        "SELECT t.id, t.name, p.body
         FROM technology_checklist_templates tct
         JOIN templates t ON t.id = tct.template_id
         JOIN template_payloads p ON p.template_id = t.id
         WHERE tct.technology_name = $1",
        &payload.name,
    )
    .await?;

    tx.commit().await.internal()?;

    let technology = sqlx::query_as::<_, ServiceTechnology>(&format!("{TECHNOLOGY_SELECT} WHERE id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    Ok(Json(technology))
}

async fn delete_technology(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let (service_id,): (Uuid,) =
        sqlx::query_as("SELECT service_id FROM service_technologies WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .internal()?
            .or_404()?;
    let host_id = service_host_id(&state.pool, service_id).await?;
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM service_technologies WHERE id = $1")
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
            "/services/{service_id}/technologies",
            axum::routing::get(list_technologies).post(create_technology),
        )
        .route(
            "/service-technologies/{id}",
            axum::routing::delete(delete_technology),
        )
}
