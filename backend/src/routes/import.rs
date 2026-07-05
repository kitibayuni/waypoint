use std::collections::HashMap;

use axum::extract::{Extension, Multipart, Path, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::authz::{require_role, EngagementRole};
use crate::import::{self, ParsedHost, ParsedImport};
use crate::routes::common::ResultExt;
use crate::routes::services::maybe_auto_checklist;
use crate::state::AppState;

async fn extract_upload(mut multipart: Multipart) -> Result<(Uuid, String), StatusCode> {
    let mut engagement_id: Option<Uuid> = None;
    let mut content: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        match field.name().unwrap_or_default() {
            "engagement_id" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                engagement_id = Uuid::parse_str(&text).ok();
            }
            "file" => {
                content = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            _ => {}
        }
    }

    let engagement_id = engagement_id.ok_or(StatusCode::BAD_REQUEST)?;
    let content = content.ok_or(StatusCode::BAD_REQUEST)?;
    Ok((engagement_id, content))
}

fn parse_source(source: &str, content: &str) -> Result<ParsedImport, StatusCode> {
    match source {
        "nmap" => import::nmap::parse(content).map_err(|_| StatusCode::BAD_REQUEST),
        "nessus" => import::nessus::parse(content).map_err(|_| StatusCode::BAD_REQUEST),
        "bloodhound" => import::bloodhound::parse(content).map_err(|_| StatusCode::BAD_REQUEST),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

/// Hostnames are matched case-insensitively (DNS names conventionally are;
/// different tools capitalize them differently -- e.g. BloodHound exports
/// upper-case FQDNs while Nmap PTR records are typically lower-case).
fn match_host(
    parsed: &ParsedHost,
    hostname_map: &HashMap<String, Uuid>,
    address_map: &HashMap<String, Uuid>,
) -> Option<Uuid> {
    if let Some(h) = &parsed.hostname
        && let Some(id) = hostname_map.get(&h.to_lowercase())
    {
        return Some(*id);
    }
    for addr in &parsed.addresses {
        if let Some(id) = address_map.get(addr) {
            return Some(*id);
        }
    }
    None
}

#[derive(Serialize)]
struct PreviewHost {
    label: String,
    hostname: Option<String>,
    os: Option<String>,
    addresses: Vec<String>,
    service_count: usize,
    action: &'static str,
}

#[derive(Serialize)]
struct ImportPreview {
    hosts: Vec<PreviewHost>,
    finding_count: usize,
    trust_relationship_count: usize,
}

async fn preview_import(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(source): Path<String>,
    multipart: Multipart,
) -> Result<Json<ImportPreview>, StatusCode> {
    let (engagement_id, content) = extract_upload(multipart).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let parsed = parse_source(&source, &content)?;

    let hostname_rows: Vec<(Uuid, Option<String>)> =
        sqlx::query_as("SELECT id, hostname FROM hosts WHERE engagement_id = $1")
            .bind(engagement_id)
            .fetch_all(&state.pool)
            .await
            .internal()?;
    let hostname_map: HashMap<String, Uuid> = hostname_rows
        .into_iter()
        .filter_map(|(id, h)| h.map(|h| (h.to_lowercase(), id)))
        .collect();

    let address_rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT h.id, host(ha.ip) FROM hosts h JOIN host_addresses ha ON ha.host_id = h.id
         WHERE h.engagement_id = $1",
    )
    .bind(engagement_id)
    .fetch_all(&state.pool)
    .await
    .internal()?;
    let address_map: HashMap<String, Uuid> =
        address_rows.into_iter().map(|(id, ip)| (ip, id)).collect();

    let hosts = parsed
        .hosts
        .iter()
        .map(|h| {
            let matched = match_host(h, &hostname_map, &address_map);
            PreviewHost {
                label: h.label.clone(),
                hostname: h.hostname.clone(),
                os: h.os.clone(),
                addresses: h.addresses.clone(),
                service_count: h.services.len(),
                action: if matched.is_some() { "merge" } else { "create" },
            }
        })
        .collect();

    Ok(Json(ImportPreview {
        hosts,
        finding_count: parsed.findings.len(),
        trust_relationship_count: parsed.trust_relationships.len(),
    }))
}

#[derive(Serialize)]
struct ImportResult {
    created_hosts: usize,
    merged_hosts: usize,
    services_added: usize,
    findings_added: usize,
    trust_relationships_added: usize,
}

async fn commit_import(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
    Path(source): Path<String>,
    multipart: Multipart,
) -> Result<Json<ImportResult>, StatusCode> {
    let (engagement_id, content) = extract_upload(multipart).await?;
    require_role(&state.pool, &user, engagement_id, EngagementRole::Tester).await?;

    let parsed = parse_source(&source, &content)?;

    let mut tx = state
        .pool
        .begin()
        .await
        .internal()?;

    let hostname_rows: Vec<(Uuid, Option<String>)> =
        sqlx::query_as("SELECT id, hostname FROM hosts WHERE engagement_id = $1")
            .bind(engagement_id)
            .fetch_all(&mut *tx)
            .await
            .internal()?;
    let mut hostname_map: HashMap<String, Uuid> = hostname_rows
        .into_iter()
        .filter_map(|(id, h)| h.map(|h| (h.to_lowercase(), id)))
        .collect();

    let address_rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT h.id, host(ha.ip) FROM hosts h JOIN host_addresses ha ON ha.host_id = h.id
         WHERE h.engagement_id = $1",
    )
    .bind(engagement_id)
    .fetch_all(&mut *tx)
    .await
    .internal()?;
    let mut address_map: HashMap<String, Uuid> =
        address_rows.into_iter().map(|(id, ip)| (ip, id)).collect();

    let mut label_to_host_id: HashMap<String, Uuid> = HashMap::new();
    let mut created_hosts = 0usize;
    let mut merged_hosts = 0usize;
    let mut services_added = 0usize;

    for h in &parsed.hosts {
        let matched = match_host(h, &hostname_map, &address_map);

        let host_id = if let Some(id) = matched {
            sqlx::query("UPDATE hosts SET hostname = COALESCE(hostname, $1), os = COALESCE(os, $2) WHERE id = $3")
                .bind(&h.hostname)
                .bind(&h.os)
                .bind(id)
                .execute(&mut *tx)
                .await
                .internal()?;
            merged_hosts += 1;
            id
        } else {
            let (new_id,): (Uuid,) = sqlx::query_as(
                "INSERT INTO hosts (engagement_id, label, hostname, os) VALUES ($1, $2, $3, $4) RETURNING id",
            )
            .bind(engagement_id)
            .bind(&h.label)
            .bind(&h.hostname)
            .bind(&h.os)
            .fetch_one(&mut *tx)
            .await
            .internal()?;
            created_hosts += 1;
            if let Some(hn) = &h.hostname {
                hostname_map.insert(hn.to_lowercase(), new_id);
            }
            new_id
        };

        label_to_host_id.insert(h.label.clone(), host_id);

        let mut mark_first_primary = matched.is_none();
        for addr in &h.addresses {
            if !address_map.contains_key(addr) {
                let is_primary = mark_first_primary;
                mark_first_primary = false;
                sqlx::query("INSERT INTO host_addresses (host_id, ip, is_primary) VALUES ($1, $2::inet, $3)")
                    .bind(host_id)
                    .bind(addr)
                    .bind(is_primary)
                    .execute(&mut *tx)
                    .await
                    .internal()?;
                address_map.insert(addr.clone(), host_id);
            }
        }

        for svc in &h.services {
            let exists: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM services WHERE host_id = $1 AND port = $2 AND protocol = $3::service_protocol",
            )
            .bind(host_id)
            .bind(svc.port)
            .bind(&svc.protocol)
            .fetch_optional(&mut *tx)
            .await
            .internal()?;

            if exists.is_none() {
                sqlx::query(
                    "INSERT INTO services (host_id, port, protocol, name, display_name, version)
                     VALUES ($1, $2, $3::service_protocol, $4, $5, $6)",
                )
                .bind(host_id)
                .bind(svc.port)
                .bind(&svc.protocol)
                .bind(&svc.name)
                .bind(&svc.product)
                .bind(&svc.version)
                .execute(&mut *tx)
                .await
                .internal()?;
                services_added += 1;

                // Bulk-imported services bypass the one-at-a-time create_service
                // handler, which is otherwise the only place that instantiates a
                // service's starter checklist -- do the same thing here so an
                // imported host doesn't silently end up missing checklists that a
                // manually-added service of the same type would have gotten.
                if let Some(name) = &svc.name {
                    maybe_auto_checklist(&mut tx, host_id, name).await?;
                }
            }
        }
    }

    let mut findings_added = 0usize;
    for f in &parsed.findings {
        let (finding_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO findings (engagement_id, title, cve, cvss_score, severity, description_md, remediation_md, status)
             VALUES ($1, $2, $3, $4::numeric, $5, $6, $7, 'open') RETURNING id",
        )
        .bind(engagement_id)
        .bind(&f.title)
        .bind(&f.cve)
        .bind(f.cvss_score)
        .bind(&f.severity)
        .bind(&f.description_md)
        .bind(&f.remediation_md)
        .fetch_one(&mut *tx)
        .await
        .internal()?;
        findings_added += 1;

        for label in &f.host_labels {
            if let Some(host_id) = label_to_host_id.get(label) {
                sqlx::query(
                    "INSERT INTO finding_hosts (finding_id, host_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(finding_id)
                .bind(host_id)
                .execute(&mut *tx)
                .await
                .internal()?;
            }
        }
    }

    let mut trust_relationships_added = 0usize;
    for t in &parsed.trust_relationships {
        if let (Some(from_id), Some(to_id)) = (
            label_to_host_id.get(&t.from_label),
            label_to_host_id.get(&t.to_label),
        ) {
            sqlx::query(
                "INSERT INTO trust_relationships (engagement_id, from_host_id, to_host_id, kind)
                 VALUES ($1, $2, $3, $4::trust_relationship_kind)",
            )
            .bind(engagement_id)
            .bind(from_id)
            .bind(to_id)
            .bind(&t.kind)
            .execute(&mut *tx)
            .await
            .internal()?;
            trust_relationships_added += 1;
        }
    }

    tx.commit()
        .await
        .internal()?;

    Ok(Json(ImportResult {
        created_hosts,
        merged_hosts,
        services_added,
        findings_added,
        trust_relationships_added,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/import/{source}/preview", axum::routing::post(preview_import))
        .route("/import/{source}/commit", axum::routing::post(commit_import))
}
