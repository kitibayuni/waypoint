use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Writes a before/after snapshot to audit_log. Best-effort: a logging
/// failure doesn't block the actual mutation, since this is an
/// observability feature, not a transactional guarantee.
pub async fn log_action<B: Serialize, A: Serialize>(
    pool: &PgPool,
    actor_id: Uuid,
    action: &str,
    subject_type: &str,
    subject_id: Uuid,
    before: Option<&B>,
    after: Option<&A>,
) {
    let before_json = before.and_then(|b| serde_json::to_value(b).ok());
    let after_json = after.and_then(|a| serde_json::to_value(a).ok());

    let _ = sqlx::query(
        "INSERT INTO audit_log (actor_id, action, subject_type, subject_id, before, after)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(actor_id)
    .bind(action)
    .bind(subject_type)
    .bind(subject_id)
    .bind(before_json)
    .bind(after_json)
    .execute(pool)
    .await;
}
