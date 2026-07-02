use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EngagementRole {
    Viewer,
    Tester,
    Lead,
}

impl EngagementRole {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "viewer" => Some(Self::Viewer),
            "tester" => Some(Self::Tester),
            "lead" => Some(Self::Lead),
            _ => None,
        }
    }
}

/// Checks that `user` holds at least `min` role on `engagement_id`.
/// Admins always pass. Returns 403 if the user is a member with an
/// insufficient role, or not a member at all.
pub async fn require_role(
    pool: &PgPool,
    user: &CurrentUser,
    engagement_id: Uuid,
    min: EngagementRole,
) -> Result<EngagementRole, StatusCode> {
    if user.is_admin {
        return Ok(EngagementRole::Lead);
    }

    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role::text FROM engagement_members WHERE engagement_id = $1 AND user_id = $2",
    )
    .bind(engagement_id)
    .bind(user.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let role = row
        .and_then(|(r,)| EngagementRole::from_str(&r))
        .ok_or(StatusCode::FORBIDDEN)?;

    if role >= min {
        Ok(role)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
