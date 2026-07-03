use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::hosts::host_engagement_id;
use crate::routes::templates::insert_checklist_from_template;
use crate::state::AppState;

const VALID_PROTOCOLS: [&str; 2] = ["tcp", "udp"];

fn valid_protocol(p: &str) -> bool {
    VALID_PROTOCOLS.contains(&p)
}

fn valid_port(p: i32) -> bool {
    (0..=65535).contains(&p)
}

fn default_protocol() -> String {
    "tcp".to_string()
}

// Controlled service-type list driving the frontend dropdown; kept in sync with
// `service_checklist_templates` (0014_service_rework.sql), which maps a subset of
// these to a starter checklist template. Not every value here has a mapping -- that's
// fine, the auto-checklist trigger is a no-op when none exists.
const VALID_SERVICE_NAMES: [&str; 16] = [
    "ssh", "ftp", "telnet", "smb", "http", "https", "rdp", "winrm", "mssql", "mysql",
    "postgresql", "ldap", "dns", "snmp", "vnc", "other",
];

fn valid_service_name(n: &str) -> bool {
    VALID_SERVICE_NAMES.contains(&n)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Service {
    id: Uuid,
    host_id: Uuid,
    port: i32,
    protocol: String,
    name: Option<String>,
    display_name: Option<String>,
    version: Option<String>,
    banner: Option<String>,
    state: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct ServiceRequest {
    port: i32,
    #[serde(default = "default_protocol")]
    protocol: String,
    name: Option<String>,
    display_name: Option<String>,
    version: Option<String>,
    banner: Option<String>,
    state: Option<String>,
}

const SERVICE_SELECT: &str = "SELECT id, host_id, port, protocol::text AS protocol, name, display_name,
    version, banner, state, created_at FROM services";

/// If `name` matches a known service type with a mapped checklist template, and this
/// host doesn't already have a checklist instantiated from that template, instantiate
/// one now (ADJUSTMENTS.txt #1/#5: "adjust the checklist based on what is logged").
async fn maybe_auto_checklist(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    host_id: Uuid,
    name: &str,
) -> Result<(), StatusCode> {
    let mapped: Option<(Uuid, String, Value)> = sqlx::query_as(
        "SELECT t.id, t.name, p.body
         FROM service_checklist_templates sct
         JOIN templates t ON t.id = sct.template_id
         JOIN template_payloads p ON p.template_id = t.id
         WHERE sct.service_name = $1",
    )
    .bind(name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some((template_id, template_name, body)) = mapped else {
        return Ok(());
    };

    let already: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM checklists WHERE host_id = $1 AND template_origin_id = $2",
    )
    .bind(host_id)
    .bind(template_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if already.is_some() {
        return Ok(());
    }

    insert_checklist_from_template(tx, Some(host_id), None, &template_name, template_id, &body).await?;
    Ok(())
}

async fn list_services(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
) -> Result<Json<Vec<Service>>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let services = sqlx::query_as::<_, Service>(&format!(
        "{SERVICE_SELECT} WHERE host_id = $1 ORDER BY port"
    ))
    .bind(host_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(services))
}

async fn create_service(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
    Json(payload): Json<ServiceRequest>,
) -> Result<Json<Service>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_protocol(&payload.protocol) || !valid_port(payload.port) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(name) = &payload.name {
        if !valid_service_name(name) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let service = sqlx::query_as::<_, Service>(&format!(
        "INSERT INTO services (host_id, port, protocol, name, display_name, version, banner, state)
         VALUES ($1, $2, $3::service_protocol, $4, $5, $6, $7, $8)
         RETURNING id, host_id, port, protocol::text AS protocol, name, display_name, version, banner, state, created_at"
    ))
    .bind(host_id)
    .bind(payload.port)
    .bind(&payload.protocol)
    .bind(&payload.name)
    .bind(&payload.display_name)
    .bind(&payload.version)
    .bind(&payload.banner)
    .bind(&payload.state)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(name) = &payload.name {
        maybe_auto_checklist(&mut tx, host_id, name).await?;
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(service))
}

async fn update_service(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((host_id, service_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<ServiceRequest>,
) -> Result<Json<Service>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_protocol(&payload.protocol) || !valid_port(payload.port) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(name) = &payload.name {
        if !valid_service_name(name) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let service = sqlx::query_as::<_, Service>(&format!(
        "UPDATE services SET port = $1, protocol = $2::service_protocol, name = $3, display_name = $4,
         version = $5, banner = $6, state = $7 WHERE id = $8 AND host_id = $9
         RETURNING id, host_id, port, protocol::text AS protocol, name, display_name, version, banner, state, created_at"
    ))
    .bind(payload.port)
    .bind(&payload.protocol)
    .bind(&payload.name)
    .bind(&payload.display_name)
    .bind(&payload.version)
    .bind(&payload.banner)
    .bind(&payload.state)
    .bind(service_id)
    .bind(host_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(service))
}

async fn delete_service(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((host_id, service_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM services WHERE id = $1 AND host_id = $2")
        .bind(service_id)
        .bind(host_id)
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
            "/hosts/{host_id}/services",
            get(list_services).post(create_service),
        )
        .route(
            "/hosts/{host_id}/services/{service_id}",
            axum::routing::put(update_service).delete(delete_service),
        )
}
