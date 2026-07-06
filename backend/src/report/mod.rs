use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fmt::Write as _;

use chrono::{DateTime, NaiveDate, Utc};
use pulldown_cmark::{html, Parser};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Markdown fields are rendered to HTML (rather than shown as raw
/// asterisks/hashes) so the client-facing PDF actually looks like a report.
/// wkhtmltopdf is invoked with --disable-javascript, so embedded <script>
/// tags in someone's notes can't execute during rendering either way.
fn render_markdown(src: &str) -> String {
    let parser = Parser::new(src);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

fn severity_rank(severity: &Option<String>) -> u8 {
    match severity.as_deref() {
        Some("critical") => 0,
        Some("high") => 1,
        Some("medium") => 2,
        Some("low") => 3,
        _ => 4,
    }
}

fn severity_color(severity: &Option<String>) -> &'static str {
    match severity.as_deref() {
        Some("critical") => "#d03b3b",
        Some("high") => "#ec835a",
        Some("medium") => "#fab219",
        Some("low") => "#0ca30c",
        _ => "#898781",
    }
}

fn jsonb_strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

#[derive(sqlx::FromRow)]
struct EngRow {
    name: String,
    status: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    global_notes_md: String,
    report_type: String,
    severity_definitions_md: String,
    client_name: String,
}

#[derive(sqlx::FromRow)]
struct ScopeRow {
    kind: String,
    value: String,
    in_scope: bool,
}

#[derive(sqlx::FromRow)]
struct FindingRow {
    title: String,
    cve: Option<String>,
    cvss_vector: Option<String>,
    cvss_score: Option<f64>,
    severity: Option<String>,
    description_md: String,
    remediation_md: String,
    poc_md: String,
    references_json: Value,
    remediation_horizon: Option<String>,
    retested_at: Option<DateTime<Utc>>,
    retested_by_name: Option<String>,
    retest_notes_md: String,
    affected_hosts: Value,
}

#[derive(sqlx::FromRow)]
struct HostBriefRow {
    id: Uuid,
    label: String,
    is_foothold: bool,
}

#[derive(sqlx::FromRow)]
struct TrustBriefRow {
    from_host_id: Uuid,
    to_host_id: Uuid,
    from_label: String,
    to_label: String,
    kind: String,
    note: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CredentialAppendixRow {
    username: String,
    domain: Option<String>,
    secret_type: String,
    origin: String,
    validated: bool,
}

#[derive(sqlx::FromRow)]
struct HostAppendixRow {
    label: String,
    os: Option<String>,
    status: String,
    addresses: Value,
}

async fn fetch_engagement_row(pool: &PgPool, engagement_id: Uuid) -> Result<Option<EngRow>, sqlx::Error> {
    sqlx::query_as::<_, EngRow>(
        "SELECT e.name, e.status::text AS status, e.start_date, e.end_date,
                e.global_notes_md, e.report_type::text AS report_type, e.severity_definitions_md,
                c.name AS client_name
         FROM engagements e JOIN clients c ON c.id = e.client_id WHERE e.id = $1",
    )
    .bind(engagement_id)
    .fetch_optional(pool)
    .await
}

async fn fetch_scope(pool: &PgPool, engagement_id: Uuid) -> Result<Vec<ScopeRow>, sqlx::Error> {
    sqlx::query_as::<_, ScopeRow>(
        "SELECT kind::text AS kind, value, in_scope FROM scope_items WHERE engagement_id = $1
         ORDER BY kind, value",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await
}

/// Findings are fetched with their affected hosts pre-aggregated (rather
/// than a separate query per finding) since a report can have dozens of
/// findings and this is a one-shot render, not an interactive page.
/// retested_by_name is aggregated via MAX (not a real aggregate -- it's a
/// single nullable FK, at most one matching row) since it comes from a
/// joined table, which Postgres's GROUP BY functional-dependency inference
/// doesn't cover the same way it does f.*'s own columns.
async fn fetch_findings(pool: &PgPool, engagement_id: Uuid) -> Result<Vec<FindingRow>, sqlx::Error> {
    let mut findings: Vec<FindingRow> = sqlx::query_as::<_, FindingRow>(
        "SELECT f.title, f.cve, f.cvss_vector, f.cvss_score::float8 AS cvss_score, f.severity,
                f.description_md, f.remediation_md, f.poc_md, f.references_json,
                f.remediation_horizon::text AS remediation_horizon, f.retested_at,
                MAX(ru.display_name) AS retested_by_name, f.retest_notes_md,
                COALESCE(jsonb_agg(DISTINCT h.label) FILTER (WHERE h.id IS NOT NULL), '[]') AS affected_hosts
         FROM findings f
         LEFT JOIN finding_hosts fh ON fh.finding_id = f.id
         LEFT JOIN hosts h ON h.id = fh.host_id
         LEFT JOIN users ru ON ru.id = f.retested_by
         WHERE f.engagement_id = $1
         GROUP BY f.id",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await?;

    findings.sort_by_key(|f| severity_rank(&f.severity));
    Ok(findings)
}

async fn fetch_hosts_brief(pool: &PgPool, engagement_id: Uuid) -> Result<Vec<HostBriefRow>, sqlx::Error> {
    sqlx::query_as::<_, HostBriefRow>(
        "SELECT id, label, is_foothold FROM hosts WHERE engagement_id = $1 ORDER BY label",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await
}

async fn fetch_trust_relationships(
    pool: &PgPool,
    engagement_id: Uuid,
) -> Result<Vec<TrustBriefRow>, sqlx::Error> {
    sqlx::query_as::<_, TrustBriefRow>(
        "SELECT tr.from_host_id, tr.to_host_id, fh.label AS from_label, th.label AS to_label,
                tr.kind::text AS kind, tr.note
         FROM trust_relationships tr
         JOIN hosts fh ON fh.id = tr.from_host_id
         JOIN hosts th ON th.id = tr.to_host_id
         WHERE tr.engagement_id = $1
         ORDER BY tr.discovered_at",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await
}

async fn fetch_credentials_appendix(
    pool: &PgPool,
    engagement_id: Uuid,
) -> Result<Vec<CredentialAppendixRow>, sqlx::Error> {
    sqlx::query_as::<_, CredentialAppendixRow>(
        "SELECT username, domain, secret_type::text AS secret_type, origin::text AS origin, validated
         FROM credentials WHERE engagement_id = $1 ORDER BY username",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await
}

async fn fetch_hosts_appendix(pool: &PgPool, engagement_id: Uuid) -> Result<Vec<HostAppendixRow>, sqlx::Error> {
    sqlx::query_as::<_, HostAppendixRow>(
        "SELECT h.label, h.os, h.status::text AS status,
                COALESCE(jsonb_agg(DISTINCT host(ha.ip)) FILTER (WHERE ha.id IS NOT NULL), '[]') AS addresses
         FROM hosts h
         LEFT JOIN host_addresses ha ON ha.host_id = h.id
         WHERE h.engagement_id = $1
         GROUP BY h.id
         ORDER BY h.label",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await
}

/// Walks outward from every foothold host via trust_relationships edges
/// (breadth-first, so branching paths all get surfaced rather than just the
/// first one found), rendering each hop as a line -- a derived report
/// section, not stored data, matching how the attack graph itself is
/// already assembled at request time. `None` if there's no foothold host or
/// no relationships recorded at all (nothing to narrate).
fn render_attack_chain(hosts: &[HostBriefRow], trusts: &[TrustBriefRow]) -> Option<String> {
    let footholds: Vec<&HostBriefRow> = hosts.iter().filter(|h| h.is_foothold).collect();
    if footholds.is_empty() || trusts.is_empty() {
        return None;
    }

    let mut out = String::new();
    for fh in &footholds {
        writeln!(out, "<h3>Starting from {}</h3>", html_escape(&fh.label)).ok();
        out.push_str("<ol>\n");

        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut queue: VecDeque<Uuid> = VecDeque::new();
        visited.insert(fh.id);
        queue.push_back(fh.id);
        let mut any_hop = false;

        while let Some(current) = queue.pop_front() {
            for t in trusts.iter().filter(|t| t.from_host_id == current) {
                any_hop = true;
                writeln!(
                    out,
                    "<li>{from} &rarr; {to} ({kind}){note}</li>",
                    from = html_escape(&t.from_label),
                    to = html_escape(&t.to_label),
                    kind = html_escape(&t.kind),
                    note = t
                        .note
                        .as_ref()
                        .map(|n| format!(" &mdash; {}", html_escape(n)))
                        .unwrap_or_default(),
                )
                .ok();
                if visited.insert(t.to_host_id) {
                    queue.push_back(t.to_host_id);
                }
            }
        }

        if !any_hop {
            out.push_str("<li><em>No recorded hops from this host.</em></li>\n");
        }
        out.push_str("</ol>\n");
    }

    Some(out)
}

/// Groups findings that have a remediation_horizon set into a short/medium/
/// long-term roadmap section. `None` if no finding has one set (an
/// engagement that never used the field shouldn't show an empty section).
fn render_recommendations(findings: &[FindingRow]) -> Option<String> {
    let mut by_horizon: BTreeMap<&str, Vec<&FindingRow>> = BTreeMap::new();
    for f in findings {
        if let Some(h) = &f.remediation_horizon {
            by_horizon.entry(h.as_str()).or_default().push(f);
        }
    }
    if by_horizon.is_empty() {
        return None;
    }

    let mut out = String::new();
    for (key, heading) in [("short", "Short-term"), ("medium", "Medium-term"), ("long", "Long-term")] {
        let Some(items) = by_horizon.get(key) else {
            continue;
        };
        writeln!(out, "<h3>{heading}</h3>").ok();
        out.push_str("<ul>\n");
        for f in items {
            writeln!(
                out,
                "<li><strong>{title}</strong>: {remediation}</li>",
                title = html_escape(&f.title),
                remediation = if f.remediation_md.trim().is_empty() {
                    "<em>see finding detail below</em>".to_string()
                } else {
                    render_markdown(&f.remediation_md)
                },
            )
            .ok();
        }
        out.push_str("</ul>\n");
    }

    Some(out)
}

const STYLE_BLOCK: &str = r#"<style>
  body { font-family: system-ui, -apple-system, "Segoe UI", sans-serif; color: #0b0b0b; margin: 2rem; }
  h1 { font-size: 1.6rem; }
  h2 { font-size: 1.2rem; border-bottom: 2px solid #c3c2b7; padding-bottom: 0.3rem; margin-top: 2.2rem; page-break-before: always; }
  h2:first-of-type { page-break-before: avoid; }
  h3 { font-size: 1.05rem; margin-bottom: 0.2rem; }
  .meta { color: #52514e; font-size: 0.9rem; }
  table { border-collapse: collapse; width: 100%; margin: 0.5rem 0 1rem; }
  th, td { text-align: left; padding: 0.3rem 0.5rem; border-bottom: 1px solid #e1e0d9; font-size: 0.9rem; }
  .severity-badge { display: inline-block; color: #fff; border-radius: 999px; padding: 0.1rem 0.6rem; font-size: 0.8rem; }
  .finding { margin-bottom: 1.5rem; padding-bottom: 1rem; border-bottom: 1px solid #e1e0d9; }
  .finding-field { margin: 0.4rem 0; }
  .finding-field-label { font-weight: 600; font-size: 0.85rem; color: #52514e; }
  .out-of-scope { color: #898781; text-decoration: line-through; }
  .retest-block { background: #f4f3ef; border-radius: 6px; padding: 0.5rem 0.75rem; margin-top: 0.5rem; }
</style>"#;

fn render_header(eng: &EngRow, finding_count: usize) -> String {
    let mut out = String::new();
    write!(
        out,
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>{title}</title>
{style}
</head>
<body>
<h1>{title}</h1>
<p class="meta">Client: {client} &middot; Status: {status}</p>

<h2>Executive Summary</h2>
<p>This report documents the findings of the assessment "{title}"
conducted for {client}. A total of {finding_count} finding(s) were identified during
this assessment.</p>
<table>
<tr><th>Severity</th><th>Count</th></tr>
"#,
        title = html_escape(&eng.name),
        client = html_escape(&eng.client_name),
        status = html_escape(&eng.status),
        style = STYLE_BLOCK,
        finding_count = finding_count,
    )
    .ok();
    out
}

/// Renders the full engagement report as a standalone HTML document, shaped
/// by `engagements.report_type` (documentation&reporting/types-of-
/// reports.txt: Vulnerability Assessment, full Penetration Test,
/// Attestation, and Post-Remediation reports have materially different
/// structures):
/// - `attestation`: exec summary + finding severity counts only.
/// - `post_remediation`: only retested findings, each with its retest
///   result alongside the original detail.
/// - `vuln_assessment`: today's full report minus Attack Chain and the
///   Credentials appendix (no exploitation occurred in this engagement type).
/// - `penetration_test` (default): everything.
///
/// Returns `None` if the engagement doesn't exist.
pub async fn render_html(pool: &PgPool, engagement_id: Uuid) -> Result<Option<String>, sqlx::Error> {
    let Some(eng) = fetch_engagement_row(pool, engagement_id).await? else {
        return Ok(None);
    };
    let report_type = eng.report_type.clone();

    let mut findings = fetch_findings(pool, engagement_id).await?;
    if report_type == "post_remediation" {
        findings.retain(|f| f.retested_at.is_some());
    }

    let mut severity_counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in &findings {
        let key = f.severity.clone().unwrap_or_else(|| "unspecified".to_string());
        *severity_counts.entry(key).or_insert(0) += 1;
    }

    let mut out = render_header(&eng, findings.len());

    for (severity, count) in &severity_counts {
        let sev_opt = if severity == "unspecified" {
            None
        } else {
            Some(severity.clone())
        };
        writeln!(
            out,
            "<tr><td><span class=\"severity-badge\" style=\"background:{color}\">{label}</span></td><td>{count}</td></tr>",
            color = severity_color(&sev_opt),
            label = html_escape(severity),
            count = count,
        )
        .ok();
    }
    out.push_str("</table>\n");

    if report_type == "attestation" {
        out.push_str("<h2>Findings Summary</h2>\n");
        if findings.is_empty() {
            out.push_str("<p><em>No findings recorded.</em></p>\n");
        } else {
            out.push_str("<table>\n<tr><th>Severity</th><th>Finding</th></tr>\n");
            for f in &findings {
                writeln!(
                    out,
                    "<tr><td><span class=\"severity-badge\" style=\"background:{color}\">{severity}</span></td><td>{title}</td></tr>",
                    color = severity_color(&f.severity),
                    severity = html_escape(f.severity.as_deref().unwrap_or("unspecified")),
                    title = html_escape(&f.title),
                )
                .ok();
            }
            out.push_str("</table>\n");
        }
        out.push_str("</body></html>\n");
        return Ok(Some(out));
    }

    let scope = fetch_scope(pool, engagement_id).await?;
    write!(
        out,
        r#"<h2>Overview of Assessment</h2>
<p>The following methodology notes were recorded over the course of the engagement:</p>
<div>{methodology}</div>

<h2>Scope &amp; Duration</h2>
<p class="meta">
{start} &rarr; {end}
</p>
<table>
<tr><th>Kind</th><th>Value</th><th>In scope</th></tr>
"#,
        methodology = if eng.global_notes_md.trim().is_empty() {
            "<p><em>No methodology notes recorded.</em></p>".to_string()
        } else {
            render_markdown(&eng.global_notes_md)
        },
        start = eng
            .start_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "(not set)".to_string()),
        end = eng
            .end_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "(not set)".to_string()),
    )
    .ok();

    for s in &scope {
        writeln!(
            out,
            "<tr class=\"{cls}\"><td>{kind}</td><td>{value}</td><td>{in_scope}</td></tr>",
            cls = if s.in_scope { "" } else { "out-of-scope" },
            kind = html_escape(&s.kind),
            value = html_escape(&s.value),
            in_scope = if s.in_scope { "yes" } else { "no (excluded)" },
        )
        .ok();
    }
    out.push_str("</table>\n");

    if report_type != "vuln_assessment" {
        let hosts = fetch_hosts_brief(pool, engagement_id).await?;
        let trusts = fetch_trust_relationships(pool, engagement_id).await?;
        if let Some(chain) = render_attack_chain(&hosts, &trusts) {
            out.push_str("<h2>Attack Chain</h2>\n");
            out.push_str(&chain);
        }
    }

    out.push_str("<h2>Vulnerabilities and Recommendations</h2>\n");
    if findings.is_empty() {
        out.push_str("<p><em>No findings recorded.</em></p>\n");
    }
    for f in &findings {
        let affected = jsonb_strings(&f.affected_hosts);
        let references = jsonb_strings(&f.references_json);

        let retest_block = if report_type == "post_remediation" {
            format!(
                r#"<div class="retest-block"><div class="finding-field-label">Retest Result</div>
                Retested {at} by {by}.<br>{notes}</div>"#,
                at = f
                    .retested_at
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
                by = html_escape(f.retested_by_name.as_deref().unwrap_or("unknown")),
                notes = if f.retest_notes_md.trim().is_empty() {
                    "<em>No retest notes recorded.</em>".to_string()
                } else {
                    render_markdown(&f.retest_notes_md)
                },
            )
        } else {
            String::new()
        };

        write!(
            out,
            r#"<div class="finding">
<h3>{title}</h3>
<p>
<span class="severity-badge" style="background:{color}">{severity}</span>
{cve}
</p>
<div class="finding-field"><div class="finding-field-label">Description</div>{description}</div>
<div class="finding-field"><div class="finding-field-label">CVSS</div>{cvss}</div>
<div class="finding-field"><div class="finding-field-label">Remediation</div>{remediation}</div>
<div class="finding-field"><div class="finding-field-label">Proof of Concept</div>{poc}</div>
<div class="finding-field"><div class="finding-field-label">Affected Systems</div>{affected}</div>
<div class="finding-field"><div class="finding-field-label">References</div>{references}</div>
{retest_block}
</div>
"#,
            title = html_escape(&f.title),
            color = severity_color(&f.severity),
            severity = html_escape(f.severity.as_deref().unwrap_or("unspecified")),
            cve = f
                .cve
                .as_ref()
                .map(|c| format!("&middot; {}", html_escape(c)))
                .unwrap_or_default(),
            description = if f.description_md.trim().is_empty() {
                "<em>none</em>".to_string()
            } else {
                render_markdown(&f.description_md)
            },
            cvss = f
                .cvss_vector
                .as_ref()
                .map(|v| {
                    format!(
                        "{} {}",
                        html_escape(v),
                        f.cvss_score.map(|s| format!("({s})")).unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| "<em>none</em>".to_string()),
            remediation = if f.remediation_md.trim().is_empty() {
                "<em>none</em>".to_string()
            } else {
                render_markdown(&f.remediation_md)
            },
            poc = if f.poc_md.trim().is_empty() {
                "<em>none</em>".to_string()
            } else {
                render_markdown(&f.poc_md)
            },
            affected = if affected.is_empty() {
                "<em>none recorded</em>".to_string()
            } else {
                affected.iter().map(|h| html_escape(h)).collect::<Vec<_>>().join(", ")
            },
            references = if references.is_empty() {
                "<em>none</em>".to_string()
            } else {
                references
                    .iter()
                    .map(|r| format!("<div>{}</div>", html_escape(r)))
                    .collect::<Vec<_>>()
                    .join("")
            },
            retest_block = retest_block,
        )
        .ok();
    }

    if let Some(rec) = render_recommendations(&findings) {
        out.push_str("<h2>Summary of Recommendations</h2>\n");
        out.push_str(&rec);
    }

    out.push_str("<h2>Appendix: Severity Ratings</h2>\n");
    out.push_str(&render_markdown(&eng.severity_definitions_md));

    if report_type != "vuln_assessment" {
        let credentials = fetch_credentials_appendix(pool, engagement_id).await?;
        out.push_str("<h2>Appendix: Compromised Credentials</h2>\n");
        if credentials.is_empty() {
            out.push_str("<p><em>None recorded.</em></p>\n");
        } else {
            out.push_str("<table>\n<tr><th>Username</th><th>Domain</th><th>Type</th><th>Origin</th><th>Validated</th></tr>\n");
            for c in &credentials {
                writeln!(
                    out,
                    "<tr><td>{username}</td><td>{domain}</td><td>{secret_type}</td><td>{origin}</td><td>{validated}</td></tr>",
                    username = html_escape(&c.username),
                    domain = c.domain.as_deref().map(html_escape).unwrap_or_default(),
                    secret_type = html_escape(&c.secret_type),
                    origin = html_escape(&c.origin),
                    validated = if c.validated { "yes" } else { "no" },
                )
                .ok();
            }
            out.push_str("</table>\n");
        }
    }

    let hosts_appendix = fetch_hosts_appendix(pool, engagement_id).await?;
    out.push_str("<h2>Appendix: Host Inventory</h2>\n");
    if hosts_appendix.is_empty() {
        out.push_str("<p><em>No hosts recorded.</em></p>\n");
    } else {
        out.push_str("<table>\n<tr><th>Host</th><th>Addresses</th><th>OS</th><th>Status</th></tr>\n");
        for h in &hosts_appendix {
            let addrs = jsonb_strings(&h.addresses);
            writeln!(
                out,
                "<tr><td>{label}</td><td>{addrs}</td><td>{os}</td><td>{status}</td></tr>",
                label = html_escape(&h.label),
                addrs = addrs.iter().map(|a| html_escape(a)).collect::<Vec<_>>().join(", "),
                os = h.os.as_deref().map(html_escape).unwrap_or_default(),
                status = html_escape(&h.status),
            )
            .ok();
        }
        out.push_str("</table>\n");
    }

    out.push_str("</body></html>\n");

    Ok(Some(out))
}
