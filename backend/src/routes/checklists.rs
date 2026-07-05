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
use crate::routes::common::{scoped_engagement_id, OptionExt, ResultExt};
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
    scoped_engagement_id(
        pool,
        "SELECT COALESCE(h.engagement_id, c.engagement_id)
         FROM checklist_items ci
         JOIN checklists c ON c.id = ci.checklist_id
         LEFT JOIN hosts h ON h.id = c.host_id
         WHERE ci.id = $1",
        item_id,
    )
    .await
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
    .internal()?;

    Ok(Json(checklists))
}

/// Every checklist across an engagement in one call, for the attack graph's
/// checklist side panel (host list + "ALL HOSTS" to-do aggregate) -- avoids an
/// N+1 of `list_host_checklists` per host on the frontend.
async fn list_engagement_checklists(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<Checklist>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let checklists = sqlx::query_as::<_, Checklist>(&format!(
        "{CHECKLIST_SELECT} LEFT JOIN hosts h ON h.id = c.host_id
         WHERE h.engagement_id = $1 OR c.engagement_id = $1
         GROUP BY c.id ORDER BY c.created_at"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(checklists))
}

/// The checklist a specific service "owns" isn't a direct FK -- it's whichever
/// checklist on the service's host was auto-instantiated from the template
/// mapped to that service's name (see `routes::services::maybe_auto_checklist`).
/// Mirrors that same lookup in reverse so the attack-graph side panel can show
/// and act on it exactly as the host's Checklists tab would.
async fn get_service_checklist(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(service_id): Path<Uuid>,
) -> Result<Json<Checklist>, StatusCode> {
    let (host_id, name): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT host_id, name FROM services WHERE id = $1",
    )
    .bind(service_id)
    .fetch_optional(&state.pool)
    .await
    .internal()?
    .or_404()?;

    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let name = name.or_404()?;

    let template_id: (Uuid,) = sqlx::query_as(
        "SELECT template_id FROM service_checklist_templates WHERE service_name = $1",
    )
    .bind(&name)
    .fetch_optional(&state.pool)
    .await
    .internal()?
    .or_404()?;

    let checklist = sqlx::query_as::<_, Checklist>(&format!(
        "{CHECKLIST_SELECT} WHERE c.host_id = $1 AND c.template_origin_id = $2 GROUP BY c.id"
    ))
    .bind(host_id)
    .bind(template_id.0)
    .fetch_optional(&state.pool)
    .await
    .internal()?
    .or_404()?;

    Ok(Json(checklist))
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
    .internal()?
    .or_404()?;

    Ok(Json(item))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hosts/{host_id}/checklists", get(list_host_checklists))
        .route(
            "/engagements/{engagement_id}/checklists",
            get(list_engagement_checklists),
        )
        .route("/services/{service_id}/checklist", get(get_service_checklist))
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
    .internal()?;

    if let Some(items) = body.get("items").and_then(|v| v.as_array()) {
        for (idx, item) in items.iter().filter_map(|v| v.as_str()).enumerate() {
            sqlx::query("INSERT INTO checklist_items (checklist_id, text, position) VALUES ($1, $2, $3)")
                .bind(checklist_id)
                .bind(item)
                .bind(idx as i32)
                .execute(&mut **tx)
                .await
                .internal()?;
        }
    }

    Ok(checklist_id)
}
