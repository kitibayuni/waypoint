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
use crate::state::AppState;

const VALID_KINDS: [&str; 4] = ["domain_trust", "admin_of", "shares_creds", "session"];

fn valid_kind(k: &str) -> bool {
    VALID_KINDS.contains(&k)
}

async fn trust_engagement_id(pool: &PgPool, id: Uuid) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>("SELECT engagement_id FROM trust_relationships WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|(id,)| id)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TrustRelationship {
    id: Uuid,
    engagement_id: Uuid,
    from_host_id: Uuid,
    from_host_label: String,
    to_host_id: Uuid,
    to_host_label: String,
    kind: String,
    direction: Option<String>,
    note: Option<String>,
    created_at: DateTime<Utc>,
}

const TRUST_SELECT: &str = "SELECT tr.id, tr.engagement_id, tr.from_host_id, fh.label AS from_host_label,
    tr.to_host_id, th.label AS to_host_label, tr.kind::text AS kind, tr.direction, tr.note, tr.created_at
    FROM trust_relationships tr
    JOIN hosts fh ON fh.id = tr.from_host_id
    JOIN hosts th ON th.id = tr.to_host_id";

#[derive(Deserialize)]
pub struct TrustRelationshipRequest {
    from_host_id: Uuid,
    to_host_id: Uuid,
    kind: String,
    direction: Option<String>,
    note: Option<String>,
}

async fn list_trusts(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<TrustRelationship>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let trusts = sqlx::query_as::<_, TrustRelationship>(&format!(
        "{TRUST_SELECT} WHERE tr.engagement_id = $1 ORDER BY tr.created_at"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(trusts))
}

async fn create_trust(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<TrustRelationshipRequest>,
) -> Result<Json<TrustRelationship>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_kind(&payload.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO trust_relationships (engagement_id, from_host_id, to_host_id, kind, direction, note)
         VALUES ($1, $2, $3, $4::trust_relationship_kind, $5, $6)
         RETURNING id",
    )
    .bind(engagement_id)
    .bind(payload.from_host_id)
    .bind(payload.to_host_id)
    .bind(&payload.kind)
    .bind(&payload.direction)
    .bind(&payload.note)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let trust = sqlx::query_as::<_, TrustRelationship>(&format!("{TRUST_SELECT} WHERE tr.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(trust))
}

async fn update_trust(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TrustRelationshipRequest>,
) -> Result<Json<TrustRelationship>, StatusCode> {
    let engagement_id = trust_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_kind(&payload.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = sqlx::query(
        "UPDATE trust_relationships SET from_host_id = $1, to_host_id = $2, kind = $3::trust_relationship_kind,
         direction = $4, note = $5 WHERE id = $6",
    )
    .bind(payload.from_host_id)
    .bind(payload.to_host_id)
    .bind(&payload.kind)
    .bind(&payload.direction)
    .bind(&payload.note)
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let trust = sqlx::query_as::<_, TrustRelationship>(&format!("{TRUST_SELECT} WHERE tr.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(trust))
}

async fn delete_trust(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = trust_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM trust_relationships WHERE id = $1")
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
            "/engagements/{engagement_id}/trust-relationships",
            get(list_trusts).post(create_trust),
        )
        .route(
            "/trust-relationships/{id}",
            axum::routing::put(update_trust).delete(delete_trust),
        )
}
