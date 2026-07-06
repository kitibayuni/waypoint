//! Small helpers shared by every route file. `ResultExt`/`OptionExt` live in
//! `crate::http_error` (used outside `routes` too, e.g. `authz.rs`) and are
//! re-exported here so existing `use crate::routes::common::{...}` imports
//! keep working; `scoped_engagement_id` is routes-specific and lives here.

use axum::http::StatusCode;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::routes::checklists::insert_checklist_from_template;

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

/// Auto-instantiates a checklist on `host_id` if `key` maps to a template via
/// `lookup_sql` (a query selecting `(template_id, template_name, body)` bound
/// to one text parameter) and one isn't already instantiated there. Shared by
/// service-name-keyed (`routes::services`) and technology-name-keyed
/// (`routes::service_technologies`) auto-instantiation, which only differ in
/// which mapping table `lookup_sql` joins through.
pub(crate) async fn instantiate_checklist_if_mapped(
    tx: &mut Transaction<'_, Postgres>,
    host_id: Uuid,
    lookup_sql: &str,
    key: &str,
) -> Result<(), StatusCode> {
    let mapped: Option<(Uuid, String, Value)> = sqlx::query_as(lookup_sql)
        .bind(key)
        .fetch_optional(&mut **tx)
        .await
        .internal()?;

    let Some((template_id, template_name, body)) = mapped else {
        return Ok(());
    };

    let already: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM checklists WHERE host_id = $1 AND template_origin_id = $2")
            .bind(host_id)
            .bind(template_id)
            .fetch_optional(&mut **tx)
            .await
            .internal()?;

    if already.is_some() {
        return Ok(());
    }

    insert_checklist_from_template(tx, Some(host_id), None, &template_name, template_id, &body).await?;
    Ok(())
}
