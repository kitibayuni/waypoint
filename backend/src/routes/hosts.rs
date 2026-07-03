use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::log_action;
use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::state::AppState;

const VALID_STATUSES: [&str; 5] = ["discovered", "enumerating", "exploited", "owned", "cleared"];

fn valid_status(s: &str) -> bool {
    VALID_STATUSES.contains(&s)
}

fn default_status() -> String {
    "discovered".to_string()
}

pub(crate) fn db_err_to_status(e: sqlx::Error) -> StatusCode {
    if let sqlx::Error::Database(db) = &e {
        // invalid_text_representation, e.g. a malformed IP address cast to ::inet
        if db.code().as_deref() == Some("22P02") {
            return StatusCode::BAD_REQUEST;
        }
    }
    StatusCode::INTERNAL_SERVER_ERROR
}

pub(crate) async fn host_engagement_id(pool: &PgPool, host_id: Uuid) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>("SELECT engagement_id FROM hosts WHERE id = $1")
        .bind(host_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(|(id,)| id)
        .ok_or(StatusCode::NOT_FOUND)
}

pub(crate) async fn get_or_create_tag(
    pool: &PgPool,
    engagement_id: Uuid,
    name: &str,
) -> Result<Uuid, sqlx::Error> {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO tags (engagement_id, name) VALUES ($1, $2)
         ON CONFLICT (engagement_id, name) DO UPDATE SET name = EXCLUDED.name
         RETURNING id",
    )
    .bind(engagement_id)
    .bind(name)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

#[derive(Serialize, Deserialize)]
pub struct TagRef {
    id: Uuid,
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddressRef {
    id: Uuid,
    ip: String,
    is_primary: bool,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Host {
    id: Uuid,
    engagement_id: Uuid,
    label: String,
    hostname: Option<String>,
    os: Option<String>,
    os_family: Option<String>,
    criticality: Option<String>,
    status: String,
    general_info_md: String,
    login_notes_md: String,
    created_at: DateTime<Utc>,
    #[sqlx(json)]
    addresses: Vec<AddressRef>,
    #[sqlx(json)]
    tags: Vec<TagRef>,
}

const HOST_SELECT: &str = "SELECT h.id, h.engagement_id, h.label, h.hostname, h.os, h.os_family,
    h.criticality, h.status::text AS status, h.general_info_md, h.login_notes_md, h.created_at,
    COALESCE(
        jsonb_agg(DISTINCT jsonb_build_object('id', ha.id, 'ip', host(ha.ip), 'is_primary', ha.is_primary))
            FILTER (WHERE ha.id IS NOT NULL),
        '[]'
    ) AS addresses,
    COALESCE(
        jsonb_agg(DISTINCT jsonb_build_object('id', t.id, 'name', t.name)) FILTER (WHERE t.id IS NOT NULL),
        '[]'
    ) AS tags
    FROM hosts h
    LEFT JOIN host_addresses ha ON ha.host_id = h.id
    LEFT JOIN host_tags ht ON ht.host_id = h.id
    LEFT JOIN tags t ON t.id = ht.tag_id";

#[derive(Deserialize)]
pub struct CreateHostRequest {
    label: String,
    hostname: Option<String>,
    os: Option<String>,
    os_family: Option<String>,
    criticality: Option<String>,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    general_info_md: String,
    #[serde(default)]
    addresses: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateHostRequest {
    label: String,
    hostname: Option<String>,
    os: Option<String>,
    os_family: Option<String>,
    criticality: Option<String>,
    status: String,
    general_info_md: String,
    #[serde(default)]
    login_notes_md: String,
}

#[derive(Deserialize)]
pub struct AddAddressRequest {
    ip: String,
    #[serde(default)]
    is_primary: bool,
}

#[derive(Deserialize)]
pub struct AddTagRequest {
    name: String,
}

async fn list_hosts(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<Host>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let hosts = sqlx::query_as::<_, Host>(&format!(
        "{HOST_SELECT} WHERE h.engagement_id = $1 GROUP BY h.id ORDER BY h.created_at"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(hosts))
}

async fn create_host(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<CreateHostRequest>,
) -> Result<Json<Host>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (host_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO hosts (engagement_id, label, hostname, os, os_family, criticality, status, general_info_md)
         VALUES ($1, $2, $3, $4, $5, $6, $7::host_status, $8)
         RETURNING id",
    )
    .bind(engagement_id)
    .bind(&payload.label)
    .bind(&payload.hostname)
    .bind(&payload.os)
    .bind(&payload.os_family)
    .bind(&payload.criticality)
    .bind(&payload.status)
    .bind(&payload.general_info_md)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for (idx, ip) in payload.addresses.iter().enumerate() {
        sqlx::query(
            "INSERT INTO host_addresses (host_id, ip, is_primary) VALUES ($1, $2::inet, $3)",
        )
        .bind(host_id)
        .bind(ip)
        .bind(idx == 0)
        .execute(&mut *tx)
        .await
        .map_err(db_err_to_status)?;
    }

    for tag_name in &payload.tags {
        let tag_id = get_or_create_tag(&state.pool, engagement_id, tag_name)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        sqlx::query("INSERT INTO host_tags (host_id, tag_id) VALUES ($1, $2)")
            .bind(host_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let host = sqlx::query_as::<_, Host>(&format!("{HOST_SELECT} WHERE h.id = $1 GROUP BY h.id"))
        .bind(host_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_action(&state.pool, user.id, "create", "host", host_id, None::<&Host>, Some(&host)).await;

    Ok(Json(host))
}

async fn get_host(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Host>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let host = sqlx::query_as::<_, Host>(&format!("{HOST_SELECT} WHERE h.id = $1 GROUP BY h.id"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(host))
}

async fn update_host(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateHostRequest>,
) -> Result<Json<Host>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_status(&payload.status) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let before = sqlx::query_as::<_, Host>(&format!("{HOST_SELECT} WHERE h.id = $1 GROUP BY h.id"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = sqlx::query(
        "UPDATE hosts SET label = $1, hostname = $2, os = $3, os_family = $4, criticality = $5,
         status = $6::host_status, general_info_md = $7, login_notes_md = $8 WHERE id = $9",
    )
    .bind(&payload.label)
    .bind(&payload.hostname)
    .bind(&payload.os)
    .bind(&payload.os_family)
    .bind(&payload.criticality)
    .bind(&payload.status)
    .bind(&payload.general_info_md)
    .bind(&payload.login_notes_md)
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let host = sqlx::query_as::<_, Host>(&format!("{HOST_SELECT} WHERE h.id = $1 GROUP BY h.id"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log_action(&state.pool, user.id, "update", "host", id, Some(&before), Some(&host)).await;

    Ok(Json(host))
}

async fn delete_host(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let before = sqlx::query_as::<_, Host>(&format!("{HOST_SELECT} WHERE h.id = $1 GROUP BY h.id"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = sqlx::query("DELETE FROM hosts WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    log_action(&state.pool, user.id, "delete", "host", id, Some(&before), None::<&Host>).await;

    Ok(StatusCode::NO_CONTENT)
}

async fn add_address(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
    Json(payload): Json<AddAddressRequest>,
) -> Result<Json<Host>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    sqlx::query("INSERT INTO host_addresses (host_id, ip, is_primary) VALUES ($1, $2::inet, $3)")
        .bind(host_id)
        .bind(&payload.ip)
        .bind(payload.is_primary)
        .execute(&state.pool)
        .await
        .map_err(db_err_to_status)?;

    let host = sqlx::query_as::<_, Host>(&format!("{HOST_SELECT} WHERE h.id = $1 GROUP BY h.id"))
        .bind(host_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(host))
}

async fn remove_address(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((host_id, address_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM host_addresses WHERE id = $1 AND host_id = $2")
        .bind(address_id)
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

async fn add_tag(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(host_id): Path<Uuid>,
    Json(payload): Json<AddTagRequest>,
) -> Result<Json<Host>, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let tag_id = get_or_create_tag(&state.pool, engagement_id, &payload.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("INSERT INTO host_tags (host_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(host_id)
        .bind(tag_id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let host = sqlx::query_as::<_, Host>(&format!("{HOST_SELECT} WHERE h.id = $1 GROUP BY h.id"))
        .bind(host_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(host))
}

async fn remove_tag(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((host_id, tag_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = host_engagement_id(&state.pool, host_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM host_tags WHERE host_id = $1 AND tag_id = $2")
        .bind(host_id)
        .bind(tag_id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        Err(StatusCode::NOT_FOUND)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Tag {
    id: Uuid,
    name: String,
}

async fn list_engagement_tags(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<Tag>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let tags = sqlx::query_as::<_, Tag>(
        "SELECT id, name FROM tags WHERE engagement_id = $1 ORDER BY name",
    )
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(tags))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/engagements/{engagement_id}/hosts",
            get(list_hosts).post(create_host),
        )
        .route("/engagements/{engagement_id}/tags", get(list_engagement_tags))
        .route(
            "/hosts/{id}",
            get(get_host).put(update_host).delete(delete_host),
        )
        .route("/hosts/{host_id}/addresses", axum::routing::post(add_address))
        .route(
            "/hosts/{host_id}/addresses/{address_id}",
            axum::routing::delete(remove_address),
        )
        .route("/hosts/{host_id}/tags", axum::routing::post(add_tag))
        .route(
            "/hosts/{host_id}/tags/{tag_id}",
            axum::routing::delete(remove_tag),
        )
}
