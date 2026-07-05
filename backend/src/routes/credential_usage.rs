use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::common::{scoped_engagement_id, ResultExt};
use crate::routes::credentials::credential_engagement_id;
use crate::state::AppState;

const VALID_RESULTS: [&str; 3] = ["works", "fails", "untested"];
const VALID_PRIVILEGES: [&str; 4] = ["user", "admin", "domain_admin", "system"];

fn valid_result(s: &str) -> bool {
    VALID_RESULTS.contains(&s)
}

fn valid_privilege(s: &str) -> bool {
    VALID_PRIVILEGES.contains(&s)
}

fn default_result() -> String {
    "untested".to_string()
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CredentialUsage {
    id: Uuid,
    credential_id: Uuid,
    host_id: Uuid,
    host_label: String,
    service_id: Option<Uuid>,
    result: String,
    privilege: Option<String>,
    created_at: DateTime<Utc>,
    tested_at: Option<DateTime<Utc>>,
}

const USAGE_SELECT: &str = "SELECT cu.id, cu.credential_id, cu.host_id, h.label AS host_label,
    cu.service_id, cu.result::text AS result, cu.privilege::text AS privilege, cu.created_at, cu.tested_at
    FROM credential_usage cu JOIN hosts h ON h.id = cu.host_id";

#[derive(Deserialize)]
pub struct CreateUsageRequest {
    host_id: Uuid,
    service_id: Option<Uuid>,
    #[serde(default = "default_result")]
    result: String,
    privilege: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateUsageRequest {
    result: String,
    privilege: Option<String>,
}

async fn list_usage(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(credential_id): Path<Uuid>,
) -> Result<Json<Vec<CredentialUsage>>, StatusCode> {
    let engagement_id = credential_engagement_id(&state.pool, credential_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let usage = sqlx::query_as::<_, CredentialUsage>(&format!(
        "{USAGE_SELECT} WHERE cu.credential_id = $1 ORDER BY cu.created_at"
    ))
    .bind(credential_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(usage))
}

async fn create_usage(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(credential_id): Path<Uuid>,
    Json(payload): Json<CreateUsageRequest>,
) -> Result<Json<CredentialUsage>, StatusCode> {
    let engagement_id = credential_engagement_id(&state.pool, credential_id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_result(&payload.result) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(p) = &payload.privilege
        && !valid_privilege(p)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO credential_usage (credential_id, host_id, service_id, result, privilege, tested_at)
         VALUES ($1, $2, $3, $4::credential_usage_result, $5::credential_privilege,
                 CASE WHEN $4 <> 'untested' THEN now() ELSE NULL END)
         RETURNING id",
    )
    .bind(credential_id)
    .bind(payload.host_id)
    .bind(payload.service_id)
    .bind(&payload.result)
    .bind(&payload.privilege)
    .fetch_one(&state.pool)
    .await
    .internal()?;

    let usage = sqlx::query_as::<_, CredentialUsage>(&format!("{USAGE_SELECT} WHERE cu.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    Ok(Json(usage))
}

async fn update_usage(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUsageRequest>,
) -> Result<Json<CredentialUsage>, StatusCode> {
    let engagement_id = usage_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    if !valid_result(&payload.result) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(p) = &payload.privilege
        && !valid_privilege(p)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = sqlx::query(
        "UPDATE credential_usage SET result = $1::credential_usage_result, privilege = $2::credential_privilege,
             tested_at = CASE WHEN $1 <> 'untested' THEN now() ELSE tested_at END
         WHERE id = $3",
    )
    .bind(&payload.result)
    .bind(&payload.privilege)
    .bind(id)
    .execute(&state.pool)
    .await
    .internal()?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let usage = sqlx::query_as::<_, CredentialUsage>(&format!("{USAGE_SELECT} WHERE cu.id = $1"))
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .internal()?;

    Ok(Json(usage))
}

async fn delete_usage(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let engagement_id = usage_engagement_id(&state.pool, id).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let result = sqlx::query("DELETE FROM credential_usage WHERE id = $1")
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

async fn usage_engagement_id(pool: &sqlx::PgPool, usage_id: Uuid) -> Result<Uuid, StatusCode> {
    scoped_engagement_id(
        pool,
        "SELECT c.engagement_id FROM credential_usage cu
         JOIN credentials c ON c.id = cu.credential_id WHERE cu.id = $1",
        usage_id,
    )
    .await
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/credentials/{credential_id}/usage",
            axum::routing::get(list_usage).post(create_usage),
        )
        .route(
            "/credential-usage/{id}",
            axum::routing::put(update_usage).delete(delete_usage),
        )
}
