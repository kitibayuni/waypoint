use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::hosts::host_engagement_id;
use crate::state::AppState;

const VALID_STATES: [&str; 4] = ["todo", "doing", "done", "na"];
const VALID_ASSESSMENTS: [&str; 3] = ["safe", "undecided", "exploit"];

fn valid_state(s: &str) -> bool {
    VALID_STATES.contains(&s)
}

fn valid_assessment(s: &str) -> bool {
    VALID_ASSESSMENTS.contains(&s)
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ChecklistItem {
    id: Uuid,
    checklist_id: Uuid,
    text: String,
    state: String,
    assessment: String,
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
            'state', ci.state::text, 'assessment', ci.assessment::text, 'position', ci.position
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
    assessment: String,
}

async fn update_checklist_item(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateChecklistItemRequest>,
) -> Result<Json<ChecklistItem>, StatusCode> {
    let engagement_id = checklist_item_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_state(&payload.state) || !valid_assessment(&payload.assessment) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let item = sqlx::query_as::<_, ChecklistItem>(
        "UPDATE checklist_items SET state = $1::checklist_item_state,
         assessment = $2::checklist_item_assessment WHERE id = $3
         RETURNING id, checklist_id, text, state::text AS state, assessment::text AS assessment, position",
    )
    .bind(&payload.state)
    .bind(&payload.assessment)
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

/// Inserts one `checklists` row + its `checklist_items` from a template payload's
/// `items` array. The only caller is `routes::services`, which uses this to
/// auto-instantiate a checklist when a service matching
/// `service_checklist_templates` is logged on a host -- the templates
/// browsing/instantiation UI itself was removed (ADJUSTMENTS.txt), but the
/// `templates`/`template_payloads` tables it reads from stay in place.
pub(crate) async fn insert_checklist_from_template(
    tx: &mut Transaction<'_, Postgres>,
    host_id: Option<Uuid>,
    checklist_engagement_id: Option<Uuid>,
    name: &str,
    template_id: Uuid,
    body: &Value,
) -> Result<Uuid, StatusCode> {
    let (checklist_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO checklists (host_id, engagement_id, name, template_origin_id)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(host_id)
    .bind(checklist_engagement_id)
    .bind(name)
    .bind(template_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(items) = body.get("items").and_then(|v| v.as_array()) {
        for (idx, item) in items.iter().filter_map(|v| v.as_str()).enumerate() {
            sqlx::query("INSERT INTO checklist_items (checklist_id, text, position) VALUES ($1, $2, $3)")
                .bind(checklist_id)
                .bind(item)
                .bind(idx as i32)
                .execute(&mut **tx)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    Ok(checklist_id)
}
