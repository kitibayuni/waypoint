use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Builds the attack-graph JSON for an engagement, shaped directly as
/// Cytoscape.js elements ({ data: {...} }) so the frontend can pass
/// `[...nodes, ...edges]` straight into `cytoscape({ elements })`.
///
/// Node types: host, credential. Edge types: trust, cred-reuse.
///
/// `as_of`, when set, replays the graph as it looked at that moment
/// (DESIGN.md §8.2): every contributing query is filtered by the timestamp
/// column that represents "this became true" rather than "this row exists"
/// (`tested_at`/`discovered_at`).
pub async fn build_graph(
    pool: &PgPool,
    engagement_id: Uuid,
    as_of: Option<DateTime<Utc>>,
) -> Result<Value, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct HostRow {
        id: Uuid,
        label: String,
        status: String,
        is_foothold: bool,
    }
    let hosts: Vec<HostRow> = sqlx::query_as(
        "SELECT id, label, status::text AS status, is_foothold FROM hosts
         WHERE engagement_id = $1 AND ($2::timestamptz IS NULL OR created_at <= $2)",
    )
    .bind(engagement_id)
    .bind(as_of)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct CredRow {
        id: Uuid,
        username: String,
        domain: Option<String>,
    }
    let credentials: Vec<CredRow> = sqlx::query_as(
        "SELECT id, username, domain FROM credentials
         WHERE engagement_id = $1 AND ($2::timestamptz IS NULL OR created_at <= $2)",
    )
    .bind(engagement_id)
    .bind(as_of)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct TrustRow {
        id: Uuid,
        from_host_id: Uuid,
        to_host_id: Uuid,
        kind: String,
    }
    let trusts: Vec<TrustRow> = sqlx::query_as(
        "SELECT id, from_host_id, to_host_id, kind::text AS kind
         FROM trust_relationships
         WHERE engagement_id = $1 AND ($2::timestamptz IS NULL OR discovered_at <= $2)",
    )
    .bind(engagement_id)
    .bind(as_of)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct UsageRow {
        id: Uuid,
        credential_id: Uuid,
        host_id: Uuid,
        privilege: Option<String>,
    }
    let usages: Vec<UsageRow> = sqlx::query_as(
        "SELECT cu.id, cu.credential_id, cu.host_id, cu.privilege::text AS privilege
         FROM credential_usage cu
         JOIN credentials c ON c.id = cu.credential_id
         WHERE c.engagement_id = $1 AND cu.result = 'works'
           AND ($2::timestamptz IS NULL OR (cu.tested_at IS NOT NULL AND cu.tested_at <= $2))",
    )
    .bind(engagement_id)
    .bind(as_of)
    .fetch_all(pool)
    .await?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for h in &hosts {
        nodes.push(json!({
            "data": {
                "id": format!("host:{}", h.id),
                "type": "host",
                "label": h.label,
                "status": h.status,
                "is_foothold": h.is_foothold,
            }
        }));
    }

    for c in &credentials {
        nodes.push(json!({
            "data": {
                "id": format!("credential:{}", c.id),
                "type": "credential",
                "label": c.username,
                "domain": c.domain,
            }
        }));
    }

    for t in &trusts {
        edges.push(json!({
            "data": {
                "id": format!("edge:trust:{}", t.id),
                "source": format!("host:{}", t.from_host_id),
                "target": format!("host:{}", t.to_host_id),
                "type": "trust",
                "label": t.kind,
            }
        }));
    }

    for u in &usages {
        edges.push(json!({
            "data": {
                "id": format!("edge:cred-reuse:{}", u.id),
                "source": format!("credential:{}", u.credential_id),
                "target": format!("host:{}", u.host_id),
                "type": "cred-reuse",
                "label": u.privilege.clone().unwrap_or_else(|| "works".to_string()),
            }
        }));
    }

    Ok(json!({ "nodes": nodes, "edges": edges }))
}
