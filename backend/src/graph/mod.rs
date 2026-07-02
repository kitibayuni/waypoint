use std::collections::HashMap;

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Builds the attack-graph JSON for an engagement, shaped directly as
/// Cytoscape.js elements ({ data: {...} }) so the frontend can pass
/// `[...nodes, ...edges]` straight into `cytoscape({ elements })`.
///
/// Node types: host, credential, observation, technique (per DESIGN.md §5).
/// Edge types: trust, cred-reuse, attack-path.
///
/// A "technique" node (and its attack-path edge) is generated for every
/// enabled attack_path_rule whose trigger observation type matches a
/// *confirmed* observation on some host — 'confirmed' is the closest
/// existing signal for "the trigger genuinely exists", and observations
/// already moved to 'remediated' or 'false_positive' no longer suggest a
/// live path (§4.9: "whose outcome isn't yet achieved").
pub async fn build_graph(pool: &PgPool, engagement_id: Uuid) -> Result<Value, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct HostRow {
        id: Uuid,
        label: String,
        status: String,
    }
    let hosts: Vec<HostRow> = sqlx::query_as(
        "SELECT id, label, status::text AS status FROM hosts WHERE engagement_id = $1",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct CredRow {
        id: Uuid,
        username: String,
        domain: Option<String>,
    }
    let credentials: Vec<CredRow> = sqlx::query_as(
        "SELECT id, username, domain FROM credentials WHERE engagement_id = $1",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct ObsRow {
        id: Uuid,
        host_id: Uuid,
        observation_type_id: Uuid,
        key: String,
        title: String,
        status: String,
        default_severity: String,
        severity_override: Option<String>,
    }
    let observations: Vec<ObsRow> = sqlx::query_as(
        "SELECT o.id, o.host_id, o.observation_type_id, ot.key, ot.title,
                o.status::text AS status, ot.default_severity, o.severity_override
         FROM observations o
         JOIN observation_types ot ON ot.id = o.observation_type_id
         JOIN hosts h ON h.id = o.host_id
         WHERE h.engagement_id = $1",
    )
    .bind(engagement_id)
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
         FROM trust_relationships WHERE engagement_id = $1",
    )
    .bind(engagement_id)
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
         WHERE c.engagement_id = $1 AND cu.result = 'works'",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await?;

    #[derive(sqlx::FromRow)]
    struct RuleRow {
        id: Uuid,
        trigger_observation_type_id: Uuid,
        technique: String,
        outcome: String,
        next_step_md: String,
        mitre_technique_id: Option<String>,
    }
    let rules: Vec<RuleRow> = sqlx::query_as(
        "SELECT id, trigger_observation_type_id, technique, outcome, next_step_md, mitre_technique_id
         FROM attack_path_rules WHERE enabled = TRUE",
    )
    .fetch_all(pool)
    .await?;

    let host_labels: HashMap<Uuid, &str> = hosts.iter().map(|h| (h.id, h.label.as_str())).collect();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut suggestions = Vec::new();

    for h in &hosts {
        nodes.push(json!({
            "data": { "id": format!("host:{}", h.id), "type": "host", "label": h.label, "status": h.status }
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

    for o in &observations {
        let severity = o
            .severity_override
            .clone()
            .unwrap_or_else(|| o.default_severity.clone());
        let observation_node_id = format!("observation:{}", o.id);

        nodes.push(json!({
            "data": {
                "id": observation_node_id,
                "type": "observation",
                "label": o.title,
                "status": o.status,
                "severity": severity,
                "parent": format!("host:{}", o.host_id),
            }
        }));

        if o.status == "confirmed" {
            for rule in rules
                .iter()
                .filter(|r| r.trigger_observation_type_id == o.observation_type_id)
            {
                let technique_node_id = format!("technique:{}:{}", rule.id, o.id);
                nodes.push(json!({
                    "data": {
                        "id": technique_node_id,
                        "type": "technique",
                        "label": rule.technique,
                        "outcome": rule.outcome,
                        "next_step_md": rule.next_step_md,
                        "mitre_technique_id": rule.mitre_technique_id,
                        "parent": observation_node_id,
                    }
                }));
                edges.push(json!({
                    "data": {
                        "id": format!("edge:attack-path:{}:{}", rule.id, o.id),
                        "source": observation_node_id,
                        "target": technique_node_id,
                        "type": "attack-path",
                    }
                }));

                suggestions.push(json!({
                    "host_id": format!("host:{}", o.host_id),
                    "host_label": host_labels.get(&o.host_id).copied().unwrap_or(""),
                    "observation_key": o.key,
                    "observation_title": o.title,
                    "technique": rule.technique,
                    "outcome": rule.outcome,
                    "next_step_md": rule.next_step_md,
                }));
            }
        }
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

    Ok(json!({ "nodes": nodes, "edges": edges, "suggestions": suggestions }))
}
