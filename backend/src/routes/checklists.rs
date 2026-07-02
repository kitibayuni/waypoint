use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::hosts::host_engagement_id;
use crate::state::AppState;

const VALID_STATES: [&str; 4] = ["todo", "doing", "done", "na"];

fn valid_state(s: &str) -> bool {
    VALID_STATES.contains(&s)
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ChecklistItem {
    id: Uuid,
    checklist_id: Uuid,
    text: String,
    state: String,
    position: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Checklist {
    id: Uuid,
    host_id: Option<Uuid>,
    engagement_id: Option<Uuid>,
    name: String,
    #[sqlx(json)]
    items: Vec<ChecklistItem>,
}

const CHECKLIST_SELECT: &str = "SELECT c.id, c.host_id, c.engagement_id, c.name,
    COALESCE(
        jsonb_agg(jsonb_build_object(
            'id', ci.id, 'checklist_id', ci.checklist_id, 'text', ci.text,
            'state', ci.state::text, 'position', ci.position
        ) ORDER BY ci.position) FILTER (WHERE ci.id IS NOT NULL),
        '[]'
    ) AS items
    FROM checklists c
    LEFT JOIN checklist_items ci ON ci.checklist_id = c.id";

async fn checklist_item_engagement_id(pool: &PgPool, item_id: Uuid) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>(
        "SELECT COALESCE(h.engagement_id, c.engagement_id)
         FROM checklist_items ci
         JOIN checklists c ON c.id = ci.checklist_id
         LEFT JOIN hosts h ON h.id = c.host_id
         WHERE ci.id = $1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|(id,)| id)
    .ok_or(StatusCode::NOT_FOUND)
}

async fn list_host_checklists(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
) -> Result<Json<Vec<Checklist>>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let checklists = sqlx::query_as::<_, Checklist>(&format!(
        "{CHECKLIST_SELECT} WHERE c.host_id = $1 GROUP BY c.id ORDER BY c.created_at"
    ))
    .bind(host_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(checklists))
}

#[derive(Deserialize)]
pub struct UpdateChecklistItemRequest {
    state: String,
}

async fn update_checklist_item(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateChecklistItemRequest>,
) -> Result<Json<ChecklistItem>, StatusCode> {
    let engagement_id = checklist_item_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_state(&payload.state) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let item = sqlx::query_as::<_, ChecklistItem>(
        "UPDATE checklist_items SET state = $1::checklist_item_state WHERE id = $2
         RETURNING id, checklist_id, text, state::text AS state, position",
    )
    .bind(&payload.state)
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(item))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hosts/{host_id}/checklists", get(list_host_checklists))
        .route("/checklist-items/{id}", axum::routing::put(update_checklist_item))
}
