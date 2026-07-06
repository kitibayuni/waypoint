use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::common::{instantiate_checklist_if_mapped, OptionExt, ResultExt};
use crate::routes::hosts::host_engagement_id;
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
// `service_checklist_templates` (0014/0016/0030_*.sql), which maps every value
// except "other" to a starter checklist template grounded in the operator's
// own notes.
const VALID_SERVICE_NAMES: [&str; 37] = [
    "ssh", "ftp", "telnet", "smb", "http", "https", "rdp", "winrm", "mssql", "mysql",
    "postgresql", "ldap", "dns", "snmp", "vnc", "nfs", "smtp", "pop3", "imap", "rsync",
    "oracle", "ipmi", "rsh", "redis", "mongodb", "elasticsearch", "cassandra", "memcached",
    "docker_api", "kubernetes_api", "mqtt", "sip", "rtsp", "ajp", "tftp", "ldaps", "other",
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
pub(crate) async fn maybe_auto_checklist(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    host_id: Uuid,
    name: &str,
) -> Result<(), StatusCode> {
    instantiate_checklist_if_mapped(
        tx,
        host_id,
        "SELECT t.id, t.name, p.body
         FROM service_checklist_templates sct
         JOIN templates t ON t.id = sct.template_id
         JOIN template_payloads p ON p.template_id = t.id
         WHERE sct.service_name = $1",
        name,
    )
    .await
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
    .internal()?;

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
    if let Some(name) = &payload.name
        && !valid_service_name(name)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .internal()?;

    let service = sqlx::query_as::<_, Service>(
        "INSERT INTO services (host_id, port, protocol, name, display_name, version, banner, state)
         VALUES ($1, $2, $3::service_protocol, $4, $5, $6, $7, $8)
         RETURNING id, host_id, port, protocol::text AS protocol, name, display_name, version, banner, state, created_at"
    )
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
    .internal()?;

    if let Some(name) = &payload.name {
        maybe_auto_checklist(&mut tx, host_id, name).await?;
    }

    tx.commit()
        .await
        .internal()?;

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
    if let Some(name) = &payload.name
        && !valid_service_name(name)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let service = sqlx::query_as::<_, Service>(
        "UPDATE services SET port = $1, protocol = $2::service_protocol, name = $3, display_name = $4,
         version = $5, banner = $6, state = $7 WHERE id = $8 AND host_id = $9
         RETURNING id, host_id, port, protocol::text AS protocol, name, display_name, version, banner, state, created_at"
    )
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
    .internal()?
    .or_404()?;

    Ok(Json(service))
}

async fn delete_service(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((host_id, service_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let mut tx = state
        .pool
        .begin()
        .await
        .internal()?;

    let deleted: Option<(Option<String>,)> = sqlx::query_as(
        "DELETE FROM services WHERE id = $1 AND host_id = $2 RETURNING name",
    )
    .bind(service_id)
    .bind(host_id)
    .fetch_optional(&mut *tx)
    .await
    .internal()?;

    let Some((name,)) = deleted else {
        return Err(StatusCode::NOT_FOUND);
    };

    // If this was the last service of that type on the host, de-instantiate the
    // checklist that was auto-created for it (ADJUSTMENTS.txt: "if a service is
    // removed, please adjust the checklist accordingly to de-instantiate").
    if let Some(name) = &name {
        let still_present: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM services WHERE host_id = $1 AND name = $2 LIMIT 1",
        )
        .bind(host_id)
        .bind(name)
        .fetch_optional(&mut *tx)
        .await
        .internal()?;

        if still_present.is_none() {
            let template_id: Option<(Uuid,)> = sqlx::query_as(
                "SELECT template_id FROM service_checklist_templates WHERE service_name = $1",
            )
            .bind(name)
            .fetch_optional(&mut *tx)
            .await
            .internal()?;

            if let Some((template_id,)) = template_id {
                sqlx::query(
                    "DELETE FROM checklists WHERE host_id = $1 AND template_origin_id = $2",
                )
                .bind(host_id)
                .bind(template_id)
                .execute(&mut *tx)
                .await
                .internal()?;
            }
        }
    }

    tx.commit()
        .await
        .internal()?;

    Ok(StatusCode::NO_CONTENT)
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
