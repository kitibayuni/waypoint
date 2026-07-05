//! Small helpers shared by every route file. `ResultExt`/`OptionExt` live in
//! `crate::http_error` (used outside `routes` too, e.g. `authz.rs`) and are
//! re-exported here so existing `use crate::routes::common::{...}` imports
//! keep working; `scoped_engagement_id` is routes-specific and lives here.

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) use crate::http_error::{OptionExt, ResultExt};

/// Every `<entity>_engagement_id` lookup (host, credential, finding, trust
/// relationship, note, checklist item, credential usage, ...) is the same
/// shape -- bind one id, fetch one engagement_id column, 404 if the row
/// doesn't exist -- differing only in the query text needed to reach it
/// (a straight lookup for most entities, a join for ones that hang off
/// another entity like checklist items or credential usage). Centralizing
/// the shape here means each of those call sites is a one-line query
/// string instead of a repeated 8-line function body.
pub(crate) async fn scoped_engagement_id(
    pool: &PgPool,
    sql: &str,
    id: Uuid,
) -> Result<Uuid, StatusCode> {
    sqlx::query_as::<_, (Uuid,)>(sql)
        .bind(id)
        .fetch_optional(pool)
        .await
        .internal()?
        .map(|(id,)| id)
        .or_404()
}
