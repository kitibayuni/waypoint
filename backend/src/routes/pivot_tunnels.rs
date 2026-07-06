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
use crate::routes::common::{scoped_engagement_id, ResultExt};
use crate::state::AppState;

const VALID_METHODS: [&str; 11] = [
    "ssh_dynamic",
    "ssh_local",
    "ssh_remote",
    "chisel",
    "ligolo",
    "sshuttle",
    "socat",
    "metasploit_autoroute",
    "dns_tunnel",
    "icmp_tunnel",
    "other",
];

fn valid_method(m: &str) -> bool {
    VALID_METHODS.contains(&m)
}

async fn pivot_engagement_id(pool: &PgPool, id: Uuid) -> Result<Uuid, StatusCode> {
    scoped_engagement_id(pool, "SELECT engagement_id FROM pivot_tunnels WHERE id = $1", id).await
}

/// How a pivot was actually established between hosts -- distinct from
/// `trust_relationships`, which records that a host can reach another, not
/// the tunnel mechanism used to get there. `to_host_id` is nullable since a
/// tunnel commonly opens up a whole subnet rather than one specific host,
/// in which case `remote_target` (a CIDR/description) is what matters.
#[derive(Serialize, sqlx::FromRow)]
pub struct PivotTunnel {
    id: Uuid,
    engagement_id: Uuid,
    from_host_id: Uuid,
    from_host_label: String,
    to_host_id: Option<Uuid>,
    to_host_label: Option<String>,
    method: String,
    local_port: Option<i32>,
    remote_target: Option<String>,
    notes_md: String,
    created_at: DateTime<Utc>,
    created_by_name: Option<String>,
}

const PIVOT_SELECT: &str = "SELECT pt.id, pt.engagement_id, pt.from_host_id, fh.label AS from_host_label,
    pt.to_host_id, th.label AS to_host_label, pt.method::text AS method, pt.local_port,
    pt.remote_target, pt.notes_md, pt.created_at, cu.display_name AS created_by_name
    FROM pivot_tunnels pt
    JOIN hosts fh ON fh.id = pt.from_host_id
    LEFT JOIN hosts th ON th.id = pt.to_host_id
    LEFT JOIN users cu ON cu.id = pt.created_by";

#[derive(Deserialize)]
pub struct PivotTunnelRequest {
    from_host_id: Uuid,
    to_host_id: Option<Uuid>,
    method: String,
    local_port: Option<i32>,
    remote_target: Option<String>,
    #[serde(default)]
    notes_md: String,
}

async fn list_pivots(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<PivotTunnel>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let pivots = sqlx::query_as::<_, PivotTunnel>(&format!(
        "{PIVOT_SELECT} WHERE pt.engagement_id = $1 ORDER BY pt.created_at"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(pivots))
}

async fn create_pivot(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<PivotTunnelRequest>,
) -> Result<Json<PivotTunnel>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_method(&payload.method) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO pivot_tunnels (engagement_id, from_host_id, to_host_id, method, local_port,
         remote_target, notes_md, created_by)
         VALUES ($1, $2, $3, $4::pivot_tunnel_method, $5, $6, $7, $8)
         RETURNING id",
    )
    .bind(engagement_id)
    .bind(payload.from_host_id)
    .bind(payload.to_host_id)
    .bind(&payload.method)
    .bind(payload.local_port)
    .bind(&payload.remote_target)
    .bind(&payload.notes_md)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await
    .internal()?;

    let pivot = sqlx::query_as::<_, PivotTunnel>(&format!("{PIVOT_SELECT} WHERE pt.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    Ok(Json(pivot))
}

async fn update_pivot(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PivotTunnelRequest>,
) -> Result<Json<PivotTunnel>, StatusCode> {
    let engagement_id = pivot_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_method(&payload.method) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = sqlx::query(
        "UPDATE pivot_tunnels SET from_host_id = $1, to_host_id = $2, method = $3::pivot_tunnel_method,
         local_port = $4, remote_target = $5, notes_md = $6 WHERE id = $7",
    )
    .bind(payload.from_host_id)
    .bind(payload.to_host_id)
    .bind(&payload.method)
    .bind(payload.local_port)
    .bind(&payload.remote_target)
    .bind(&payload.notes_md)
    .bind(id)
    .execute(&state.pool)
    .await
    .internal()?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let pivot = sqlx::query_as::<_, PivotTunnel>(&format!("{PIVOT_SELECT} WHERE pt.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    Ok(Json(pivot))
}

async fn delete_pivot(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = pivot_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM pivot_tunnels WHERE id = $1")
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
            "/engagements/{engagement_id}/pivot-tunnels",
            get(list_pivots).post(create_pivot),
        )
        .route(
            "/pivot-tunnels/{id}",
            axum::routing::put(update_pivot).delete(delete_pivot),
        )
}
