use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::routes::common::{OptionExt, ResultExt};
use crate::state::AppState;

const VALID_ROLES: [&str; 3] = ["viewer", "tester", "lead"];

fn valid_role(r: &str) -> bool {
    VALID_ROLES.contains(&r)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Member {
    user_id: Uuid,
    email: String,
    display_name: String,
    role: String,
    added_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    email: String,
    role: String,
}

#[derive(Deserialize)]
pub struct UpdateMemberRoleRequest {
    role: String,
}

const MEMBER_SELECT: &str = "SELECT m.user_id, u.email, u.display_name, m.role::text AS role, m.added_at
     FROM engagement_members m JOIN users u ON u.id = m.user_id";

async fn list_members(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
) -> Result<Json<Vec<Member>>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Viewer).await?;

    let members = sqlx::query_as::<_, Member>(&format!(
        "{MEMBER_SELECT} WHERE m.engagement_id = $1 ORDER BY m.added_at"
    ))
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;

    Ok(Json(members))
}

async fn add_member(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(engagement_id): Path<Uuid>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<Member>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Lead).await?;

    if !valid_role(&payload.role) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let target: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.pool)
        .await
        .internal()?;
    let target_id = target.or_404()?.0;

    sqlx::query(
        "INSERT INTO engagement_members (engagement_id, user_id, role)
         VALUES ($1, $2, $3::engagement_role)",
    )
    .bind(engagement_id)
    .bind(target_id)
    .bind(&payload.role)
    .execute(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db) if db.is_unique_violation() => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;

    let member = sqlx::query_as::<_, Member>(&format!(
        "{MEMBER_SELECT} WHERE m.engagement_id = $1 AND m.user_id = $2"
    ))
    .bind(engagement_id)
    .bind(target_id)
    .fetch_one(&state.pool)
    .await
    .internal()?;

    Ok(Json(member))
}

async fn update_member_role(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((engagement_id, target_user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateMemberRoleRequest>,
) -> Result<Json<Member>, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Lead).await?;

    if !valid_role(&payload.role) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = sqlx::query(
        "UPDATE engagement_members SET role = $1::engagement_role
         WHERE engagement_id = $2 AND user_id = $3",
    )
    .bind(&payload.role)
    .bind(engagement_id)
    .bind(target_user_id)
    .execute(&state.pool)
    .await
    .internal()?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    let member = sqlx::query_as::<_, Member>(&format!(
        "{MEMBER_SELECT} WHERE m.engagement_id = $1 AND m.user_id = $2"
    ))
    .bind(engagement_id)
    .bind(target_user_id)
    .fetch_one(&state.pool)
    .await
    .internal()?;

    Ok(Json(member))
}

async fn remove_member(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path((engagement_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    require_role(&state.pool, &user, engagement_id, EngagementRole::Lead).await?;

    let result = sqlx::query(
        "DELETE FROM engagement_members WHERE engagement_id = $1 AND user_id = $2",
    )
    .bind(engagement_id)
    .bind(target_user_id)
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
            "/engagements/{engagement_id}/members",
            get(list_members).post(add_member),
        )
        .route(
            "/engagements/{engagement_id}/members/{user_id}",
            axum::routing::put(update_member_role).delete(remove_member),
        )
}
