use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::hosts::{get_or_create_tag, host_engagement_id};
use crate::state::AppState;

const VALID_KINDS: [&str; 5] = ["host", "checklist", "finding", "note", "engagement"];
const VALID_SUBJECT_TYPES: [&str; 5] = ["engagement", "host", "finding", "observation", "credential"];

fn valid_kind(k: &str) -> bool {
    VALID_KINDS.contains(&k)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Template {
    id: Uuid,
    kind: String,
    name: String,
    description: Option<String>,
    owner_id: Option<Uuid>,
    is_shared: bool,
    created_at: DateTime<Utc>,
    body: Value,
}

const TEMPLATE_SELECT: &str = "SELECT t.id, t.kind::text AS kind, t.name, t.description, t.owner_id,
    t.is_shared, t.created_at, p.body
    FROM templates t JOIN template_payloads p ON p.template_id = t.id";

#[derive(Deserialize)]
pub struct TemplateRequest {
    kind: String,
    name: String,
    description: Option<String>,
    #[serde(default = "default_true")]
    is_shared: bool,
    #[serde(default)]
    body: Value,
}

fn default_true() -> bool {
    true
}

fn normalize_body(v: Value) -> Value {
    if v.is_null() {
        serde_json::json!({})
    } else {
        v
    }
}

async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<Template>>, StatusCode> {
    let templates = sqlx::query_as::<_, Template>(&format!(
        "{TEMPLATE_SELECT} ORDER BY t.kind, t.name"
    ))
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(templates))
}

async fn create_template(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Json(payload): Json<TemplateRequest>,
) -> Result<Json<Template>, StatusCode> {
    if !valid_kind(&payload.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO templates (kind, name, description, owner_id, is_shared)
         VALUES ($1::template_kind, $2, $3, $4, $5) RETURNING id",
    )
    .bind(&payload.kind)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(user.id)
    .bind(payload.is_shared)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("INSERT INTO template_payloads (template_id, body) VALUES ($1, $2)")
        .bind(id)
        .bind(normalize_body(payload.body))
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = sqlx::query_as::<_, Template>(&format!("{TEMPLATE_SELECT} WHERE t.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(template))
}

async fn get_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Template>, StatusCode> {
    let template = sqlx::query_as::<_, Template>(&format!("{TEMPLATE_SELECT} WHERE t.id = $1"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(template))
}

async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<TemplateRequest>,
) -> Result<Json<Template>, StatusCode> {
    if !valid_kind(&payload.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = sqlx::query(
        "UPDATE templates SET kind = $1::template_kind, name = $2, description = $3, is_shared = $4 WHERE id = $5",
    )
    .bind(&payload.kind)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(payload.is_shared)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    sqlx::query("UPDATE template_payloads SET body = $1 WHERE template_id = $2")
        .bind(normalize_body(payload.body))
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = sqlx::query_as::<_, Template>(&format!("{TEMPLATE_SELECT} WHERE t.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(template))
}

async fn delete_template(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM templates WHERE id = $1")
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

#[derive(Deserialize)]
pub struct InstantiateRequest {
    engagement_id: Option<Uuid>,
    host_id: Option<Uuid>,
    client_id: Option<Uuid>,
    name: Option<String>,
    hostname: Option<String>,
    os: Option<String>,
    subject_type: Option<String>,
    subject_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct InstantiateResponse {
    kind: String,
    id: Uuid,
    engagement_id: Uuid,
}

async fn instantiate_host(
    tx: &mut Transaction<'_, Postgres>,
    pool: &PgPool,
    template_id: Uuid,
    template_name: &str,
    body: &Value,
    req: &InstantiateRequest,
    user: &CurrentUser,
) -> Result<(Uuid, Uuid), StatusCode> {
    let engagement_id = req.engagement_id.ok_or(StatusCode::BAD_REQUEST)?;
    let label = req.name.clone().unwrap_or_else(|| template_name.to_string());

    let (host_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO hosts (engagement_id, label, hostname, os, status, template_origin_id)
         VALUES ($1, $2, $3, $4, 'discovered', $5) RETURNING id",
    )
    .bind(engagement_id)
    .bind(&label)
    .bind(&req.hostname)
    .bind(&req.os)
    .bind(template_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(tags) = body.get("tags").and_then(|v| v.as_array()) {
        for tag in tags.iter().filter_map(|v| v.as_str()) {
            let tag_id = get_or_create_tag(pool, engagement_id, tag)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            sqlx::query("INSERT INTO host_tags (host_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(host_id)
                .bind(tag_id)
                .execute(&mut **tx)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    if let Some(services) = body.get("services").and_then(|v| v.as_array()) {
        for svc in services {
            let port = svc.get("port").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let protocol = svc.get("protocol").and_then(|v| v.as_str()).unwrap_or("tcp");
            let name = svc.get("name").and_then(|v| v.as_str());
            let product = svc.get("product").and_then(|v| v.as_str());
            sqlx::query(
                "INSERT INTO services (host_id, port, protocol, name, product) VALUES ($1, $2, $3::service_protocol, $4, $5)",
            )
            .bind(host_id)
            .bind(port)
            .bind(protocol)
            .bind(name)
            .bind(product)
            .execute(&mut **tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    if let Some(checklists) = body.get("checklists").and_then(|v| v.as_array()) {
        for cl in checklists {
            let cl_name = cl.get("name").and_then(|v| v.as_str()).unwrap_or("Checklist");
            let (checklist_id,): (Uuid,) = sqlx::query_as(
                "INSERT INTO checklists (host_id, name, template_origin_id) VALUES ($1, $2, $3) RETURNING id",
            )
            .bind(host_id)
            .bind(cl_name)
            .bind(template_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if let Some(items) = cl.get("items").and_then(|v| v.as_array()) {
                for (idx, item) in items.iter().filter_map(|v| v.as_str()).enumerate() {
                    sqlx::query(
                        "INSERT INTO checklist_items (checklist_id, text, position) VALUES ($1, $2, $3)",
                    )
                    .bind(checklist_id)
                    .bind(item)
                    .bind(idx as i32)
                    .execute(&mut **tx)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                }
            }
        }
    }

    if let Some(notes) = body.get("notes").and_then(|v| v.as_array()) {
        for note in notes {
            let title = note.get("title").and_then(|v| v.as_str());
            let body_md = note.get("body_md").and_then(|v| v.as_str()).unwrap_or("");
            sqlx::query(
                "INSERT INTO notes (engagement_id, subject_type, subject_id, title, body_md, created_by)
                 VALUES ($1, 'host'::note_subject_type, $2, $3, $4, $5)",
            )
            .bind(engagement_id)
            .bind(host_id)
            .bind(title)
            .bind(body_md)
            .bind(user.id)
            .execute(&mut **tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    Ok((host_id, engagement_id))
}

async fn instantiate_checklist(
    tx: &mut Transaction<'_, Postgres>,
    pool: &PgPool,
    template_id: Uuid,
    template_name: &str,
    body: &Value,
    req: &InstantiateRequest,
    user: &CurrentUser,
) -> Result<(Uuid, Uuid), StatusCode> {
    let (host_id, engagement_id) = match (req.host_id, req.engagement_id) {
        (Some(h), _) => {
            let eng = host_engagement_id(pool, h).await?;
            (Some(h), eng)
        }
        (None, Some(e)) => (None, e),
        (None, None) => return Err(StatusCode::BAD_REQUEST),
    };

    require_role(pool, user, engagement_id, EngagementRole::Tester).await?;

    let name = req
        .name
        .clone()
        .or_else(|| body.get("name").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| template_name.to_string());

    let checklist_engagement_id = if host_id.is_some() { None } else { Some(engagement_id) };

    let checklist_id =
        insert_checklist_from_template(tx, host_id, checklist_engagement_id, &name, template_id, body)
            .await?;

    Ok((checklist_id, engagement_id))
}

/// Shared checklist-instantiation core: inserts one `checklists` row + its
/// `checklist_items` from a template payload's `items` array. Used both by the
/// explicit "instantiate template" flow above and by the automatic
/// service-type -> checklist trigger in `routes::services`.
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

async fn instantiate_finding(
    tx: &mut Transaction<'_, Postgres>,
    body: &Value,
    req: &InstantiateRequest,
) -> Result<(Uuid, Uuid), StatusCode> {
    let engagement_id = req.engagement_id.ok_or(StatusCode::BAD_REQUEST)?;

    let title = req
        .name
        .clone()
        .or_else(|| body.get("title").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| "Untitled Finding".to_string());
    let severity = body.get("severity").and_then(|v| v.as_str());
    let description_md = body.get("description_md").and_then(|v| v.as_str()).unwrap_or("");
    let remediation_md = body.get("remediation_md").and_then(|v| v.as_str()).unwrap_or("");
    let poc_md = body.get("poc_md").and_then(|v| v.as_str()).unwrap_or("");
    let references_json = body.get("references_json").cloned().unwrap_or_else(|| serde_json::json!([]));

    let (finding_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO findings (engagement_id, title, severity, description_md, remediation_md, poc_md, references_json)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(engagement_id)
    .bind(&title)
    .bind(severity)
    .bind(description_md)
    .bind(remediation_md)
    .bind(poc_md)
    .bind(&references_json)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((finding_id, engagement_id))
}

async fn instantiate_note(
    tx: &mut Transaction<'_, Postgres>,
    body: &Value,
    req: &InstantiateRequest,
    user: &CurrentUser,
) -> Result<(Uuid, Uuid), StatusCode> {
    let engagement_id = req.engagement_id.ok_or(StatusCode::BAD_REQUEST)?;
    let subject_type = req.subject_type.as_deref().ok_or(StatusCode::BAD_REQUEST)?;
    let subject_id = req.subject_id.ok_or(StatusCode::BAD_REQUEST)?;

    if !VALID_SUBJECT_TYPES.contains(&subject_type) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let title = req
        .name
        .clone()
        .or_else(|| body.get("title").and_then(|v| v.as_str()).map(String::from));
    let body_md = body.get("body_md").and_then(|v| v.as_str()).unwrap_or("");

    let (note_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO notes (engagement_id, subject_type, subject_id, title, body_md, created_by)
         VALUES ($1, $2::note_subject_type, $3, $4, $5, $6) RETURNING id",
    )
    .bind(engagement_id)
    .bind(subject_type)
    .bind(subject_id)
    .bind(&title)
    .bind(body_md)
    .bind(user.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((note_id, engagement_id))
}

async fn instantiate_engagement(
    tx: &mut Transaction<'_, Postgres>,
    body: &Value,
    req: &InstantiateRequest,
    user: &CurrentUser,
) -> Result<(Uuid, Uuid), StatusCode> {
    let client_id = req.client_id.ok_or(StatusCode::BAD_REQUEST)?;
    let name = req.name.clone().ok_or(StatusCode::BAD_REQUEST)?;

    let (engagement_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO engagements (client_id, name, status, created_by) VALUES ($1, $2, 'planning', $3) RETURNING id",
    )
    .bind(client_id)
    .bind(&name)
    .bind(user.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("INSERT INTO engagement_members (engagement_id, user_id, role) VALUES ($1, $2, 'lead')")
        .bind(engagement_id)
        .bind(user.id)
        .execute(&mut **tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(findings) = body.get("standard_findings").and_then(|v| v.as_array()) {
        for f in findings {
            let title = f.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Finding");
            let severity = f.get("severity").and_then(|v| v.as_str());
            let description_md = f.get("description_md").and_then(|v| v.as_str()).unwrap_or("");
            let remediation_md = f.get("remediation_md").and_then(|v| v.as_str()).unwrap_or("");
            let poc_md = f.get("poc_md").and_then(|v| v.as_str()).unwrap_or("");
            let references_json = f.get("references_json").cloned().unwrap_or_else(|| serde_json::json!([]));

            sqlx::query(
                "INSERT INTO findings (engagement_id, title, severity, description_md, remediation_md, poc_md, references_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(engagement_id)
            .bind(title)
            .bind(severity)
            .bind(description_md)
            .bind(remediation_md)
            .bind(poc_md)
            .bind(&references_json)
            .execute(&mut **tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    Ok((engagement_id, engagement_id))
}

async fn instantiate_template(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(template_id): Path<Uuid>,
    Json(req): Json<InstantiateRequest>,
) -> Result<Json<InstantiateResponse>, StatusCode> {
    let template = sqlx::query_as::<_, Template>(&format!("{TEMPLATE_SELECT} WHERE t.id = $1"))
        .bind(template_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // RBAC for kinds whose target engagement is already known up front.
    // instantiate_checklist resolves and checks its own engagement_id
    // (host_id may be the only thing supplied); instantiate_engagement
    // creates a brand-new engagement, so there's nothing to check yet,
    // matching create_engagement's existing behavior.
    match template.kind.as_str() {
        "host" | "finding" | "note" => {
            let engagement_id = req.engagement_id.ok_or(StatusCode::BAD_REQUEST)?;
            require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;
        }
        _ => {}
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (id, engagement_id) = match template.kind.as_str() {
        "host" => {
            instantiate_host(&mut tx, &state.pool, template.id, &template.name, &template.body, &req, &user).await?
        }
        "checklist" => {
            instantiate_checklist(&mut tx, &state.pool, template.id, &template.name, &template.body, &req, &user)
                .await?
        }
        "finding" => instantiate_finding(&mut tx, &template.body, &req).await?,
        "note" => instantiate_note(&mut tx, &template.body, &req, &user).await?,
        "engagement" => instantiate_engagement(&mut tx, &template.body, &req, &user).await?,
        _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(InstantiateResponse {
        kind: template.kind,
        id,
        engagement_id,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/templates", get(list_templates).post(create_template))
        .route(
            "/templates/{id}",
            get(get_template).put(update_template).delete(delete_template),
        )
        .route("/templates/{id}/instantiate", axum::routing::post(instantiate_template))
}
