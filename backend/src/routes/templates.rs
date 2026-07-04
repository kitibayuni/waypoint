use axum::http::StatusCode;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// The Templates browsing/instantiation UI was removed (ADJUSTMENTS.txt), but the
/// underlying `templates`/`template_payloads` tables stay in place: they're still
/// referenced by `service_checklist_templates.template_id` and
/// `checklists.template_origin_id`, and this function is the one piece of that old
/// feature still in active use -- it's called directly by `routes::services` to
/// auto-instantiate a checklist when a matching service is logged.
///
/// Inserts one `checklists` row + its `checklist_items` from a template payload's
/// `items` array.
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
