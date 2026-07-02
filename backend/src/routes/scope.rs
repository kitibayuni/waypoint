use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

const VALID_KINDS: [&str; 6] = ["ip", "cidr", "domain", "url", "asn", "exclusion"];

fn valid_kind(k: &str) -> bool {
    VALID_KINDS.contains(&k)
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ScopeItem {
    id: Uuid,
    engagement_id: Uuid,
    kind: String,
    value: String,
    in_scope: bool,
    note: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ScopeItemRequest {
    kind: String,
    value: String,
    #[serde(default = "default_true")]
    in_scope: bool,
    note: Option<String>,
}

async fn list_scope(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<ScopeItem>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let items = sqlx::query_as::<_, ScopeItem>(
        "SELECT id, engagement_id, kind::text AS kind, value, in_scope, note, created_at
         FROM scope_items WHERE engagement_id = $1 ORDER BY created_at",
    )
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(items))
}

async fn create_scope_item(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<ScopeItemRequest>,
) -> Result<Json<ScopeItem>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_kind(&payload.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let item = sqlx::query_as::<_, ScopeItem>(
        "INSERT INTO scope_items (engagement_id, kind, value, in_scope, note)
         VALUES ($1, $2::scope_item_kind, $3, $4, $5)
         RETURNING id, engagement_id, kind::text AS kind, value, in_scope, note, created_at",
    )
    .bind(engagement_id)
    .bind(&payload.kind)
    .bind(&payload.value)
    .bind(payload.in_scope)
    .bind(&payload.note)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(item))
}

async fn update_scope_item(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((engagement_id, scope_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<ScopeItemRequest>,
) -> Result<Json<ScopeItem>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_kind(&payload.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let item = sqlx::query_as::<_, ScopeItem>(
        "UPDATE scope_items SET kind = $1::scope_item_kind, value = $2, in_scope = $3, note = $4
         WHERE id = $5 AND engagement_id = $6
         RETURNING id, engagement_id, kind::text AS kind, value, in_scope, note, created_at",
    )
    .bind(&payload.kind)
    .bind(&payload.value)
    .bind(payload.in_scope)
    .bind(&payload.note)
    .bind(scope_id)
    .bind(engagement_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

async fn delete_scope_item(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((engagement_id, scope_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM scope_items WHERE id = $1 AND engagement_id = $2")
        .bind(scope_id)
        .bind(engagement_id)
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
            "/engagements/{engagement_id}/scope",
            get(list_scope).post(create_scope_item),
        )
        .route(
            "/engagements/{engagement_id}/scope/{scope_id}",
            axum::routing::put(update_scope_item).delete(delete_scope_item),
        )
}
