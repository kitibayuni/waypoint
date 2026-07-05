use std::collections::BTreeMap;
use std::fmt::Write as _;

use chrono::NaiveDate;
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

struct EngRow {
    name: String,
    status: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    global_notes_md: String,
    client_name: String,
}

struct ScopeRow {
    kind: String,
    value: String,
    in_scope: bool,
}

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
    affected_hosts: Value,
}

/// Renders the full engagement report as a standalone HTML document
/// (Executive Summary, Overview/Methodology, Scope & Duration, Findings
/// grouped by severity) per notes/on-reporting.txt's structure. Returns
/// `None` if the engagement doesn't exist.
pub async fn render_html(pool: &PgPool, engagement_id: Uuid) -> Result<Option<String>, sqlx::Error> {
    let eng = sqlx::query_as::<_, (String, String, Option<NaiveDate>, Option<NaiveDate>, String, String)>(
        "SELECT e.name, e.status::text AS status, e.start_date, e.end_date,
                e.global_notes_md, c.name AS client_name
         FROM engagements e JOIN clients c ON c.id = e.client_id WHERE e.id = $1",
    )
    .bind(engagement_id)
    .fetch_optional(pool)
    .await?
    .map(|(name, status, start_date, end_date, global_notes_md, client_name)| EngRow {
        name,
        status,
        start_date,
        end_date,
        global_notes_md,
        client_name,
    });

    let Some(eng) = eng else {
        return Ok(None);
    };

    let scope: Vec<ScopeRow> = sqlx::query_as::<_, (String, String, bool)>(
        "SELECT kind::text AS kind, value, in_scope FROM scope_items WHERE engagement_id = $1
         ORDER BY kind, value",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(kind, value, in_scope)| ScopeRow { kind, value, in_scope })
    .collect();

    let findings: Vec<FindingRow> = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<String>,
            Option<f64>,
            Option<String>,
            String,
            String,
            String,
            Value,
            Value,
        ),
    >(
        "SELECT f.title, f.cve, f.cvss_vector, f.cvss_score::float8 AS cvss_score, f.severity,
                f.description_md, f.remediation_md, f.poc_md, f.references_json,
                COALESCE(jsonb_agg(DISTINCT h.label) FILTER (WHERE h.id IS NOT NULL), '[]') AS affected_hosts
         FROM findings f
         LEFT JOIN finding_hosts fh ON fh.finding_id = f.id
         LEFT JOIN hosts h ON h.id = fh.host_id
         WHERE f.engagement_id = $1
         GROUP BY f.id",
    )
    .bind(engagement_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(
        |(
            title,
            cve,
            cvss_vector,
            cvss_score,
            severity,
            description_md,
            remediation_md,
            poc_md,
            references_json,
            affected_hosts,
        )| FindingRow {
            title,
            cve,
            cvss_vector,
            cvss_score,
            severity,
            description_md,
            remediation_md,
            poc_md,
            references_json,
            affected_hosts,
        },
    )
    .collect();

    let mut findings = findings;
    findings.sort_by_key(|f| severity_rank(&f.severity));

    let mut severity_counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in &findings {
        let key = f.severity.clone().unwrap_or_else(|| "unspecified".to_string());
        *severity_counts.entry(key).or_insert(0) += 1;
    }

    let mut out = String::new();

    write!(
        out,
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
  body {{ font-family: system-ui, -apple-system, "Segoe UI", sans-serif; color: #0b0b0b; margin: 2rem; }}
  h1 {{ font-size: 1.6rem; }}
  h2 {{ font-size: 1.2rem; border-bottom: 2px solid #c3c2b7; padding-bottom: 0.3rem; margin-top: 2.2rem; page-break-before: always; }}
  h2:first-of-type {{ page-break-before: avoid; }}
  h3 {{ font-size: 1.05rem; margin-bottom: 0.2rem; }}
  .meta {{ color: #52514e; font-size: 0.9rem; }}
  table {{ border-collapse: collapse; width: 100%; margin: 0.5rem 0 1rem; }}
  th, td {{ text-align: left; padding: 0.3rem 0.5rem; border-bottom: 1px solid #e1e0d9; font-size: 0.9rem; }}
  .severity-badge {{ display: inline-block; color: #fff; border-radius: 999px; padding: 0.1rem 0.6rem; font-size: 0.8rem; }}
  .finding {{ margin-bottom: 1.5rem; padding-bottom: 1rem; border-bottom: 1px solid #e1e0d9; }}
  .finding-field {{ margin: 0.4rem 0; }}
  .finding-field-label {{ font-weight: 600; font-size: 0.85rem; color: #52514e; }}
  .out-of-scope {{ color: #898781; text-decoration: line-through; }}
</style>
</head>
<body>
<h1>{title}</h1>
<p class="meta">Client: {client} &middot; Status: {status}</p>

<h2>Executive Summary</h2>
<p>This report documents the findings of the penetration testing engagement "{title}"
conducted for {client}. A total of {finding_count} finding(s) were identified during
this assessment.</p>
<table>
<tr><th>Severity</th><th>Count</th></tr>
"#,
        title = html_escape(&eng.name),
        client = html_escape(&eng.client_name),
        status = html_escape(&eng.status),
        finding_count = findings.len(),
    )
    .ok();

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

    out.push_str("<h2>Vulnerabilities and Recommendations</h2>\n");
    if findings.is_empty() {
        out.push_str("<p><em>No findings recorded.</em></p>\n");
    }
    for f in &findings {
        let affected: Vec<String> = f
            .affected_hosts
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let references: Vec<String> = f
            .references_json
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

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
        )
        .ok();
    }

    out.push_str("</body></html>\n");

    Ok(Some(out))
}
