use std::collections::HashMap;

use super::{ParsedFinding, ParsedHost, ParsedImport, ParsedService};

/// OpenVAS/Greenbone's `severity` on a result is a CVSS-like 0-10 float
/// (unlike Nessus's 0-4 integer scale) -- bucketed via the standard CVSS
/// severity thresholds. 0 is informational (e.g. "port is open") and
/// contributes no finding, matching how Nessus severity 0 is handled.
fn openvas_severity_to_ours(severity: f64) -> Option<&'static str> {
    if severity <= 0.0 {
        None
    } else if severity < 4.0 {
        Some("low")
    } else if severity < 7.0 {
        Some("medium")
    } else if severity < 9.0 {
        Some("high")
    } else {
        Some("critical")
    }
}

/// Parses an OpenVAS/Greenbone GMP `<get_reports>` results export
/// (`<report><results><result>...</result></results></report>`). Written
/// against the commonly-documented GMP results schema -- unlike the Nmap/
/// Nessus parsers, this has not been validated against a real OpenVAS
/// export, so field-name mismatches against a specific GVM version are
/// possible; treat a first real import as a trial run and check the
/// preview counts look sane before committing.
///
/// Findings are grouped by (NVT name, severity) the same way Nessus's
/// ReportItems are, so one vulnerability type produces one finding with
/// every affected host attached rather than a duplicate per host.
pub fn parse(xml: &str) -> Result<ParsedImport, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;

    let mut hosts_by_ip: HashMap<String, ParsedHost> = HashMap::new();
    let mut host_order: Vec<String> = Vec::new();
    let mut findings: HashMap<(String, String), ParsedFinding> = HashMap::new();

    for result in doc.descendants().filter(|n| n.has_tag_name("result")) {
        let ip = result
            .children()
            .find(|n| n.has_tag_name("host"))
            .and_then(|n| n.text().or_else(|| n.attribute("asset_id")))
            .unwrap_or("")
            .trim()
            .to_string();
        if ip.is_empty() {
            continue;
        }

        let entry = hosts_by_ip.entry(ip.clone()).or_insert_with(|| {
            host_order.push(ip.clone());
            ParsedHost {
                label: ip.clone(),
                hostname: None,
                os: None,
                addresses: vec![ip.clone()],
                services: Vec::new(),
            }
        });

        let port_text = result.children().find(|n| n.has_tag_name("port")).and_then(|n| n.text());
        let mut seen_ports: std::collections::HashSet<(i32, String)> =
            entry.services.iter().map(|s| (s.port, s.protocol.clone())).collect();
        // OpenVAS reports ports as "443/tcp"; "general/tcp" and similar
        // non-port-specific pseudo-services carry no useful port number.
        if let Some(port_text) = port_text
            && let Some((port_str, protocol)) = port_text.split_once('/')
            && let Ok(port) = port_str.parse::<i32>()
        {
            let protocol = protocol.trim().to_lowercase();
            if seen_ports.insert((port, protocol.clone())) {
                entry.services.push(ParsedService {
                    port,
                    protocol,
                    name: None,
                    product: None,
                    version: None,
                });
            }
        }

        let nvt = result.children().find(|n| n.has_tag_name("nvt"));
        let plugin_name = nvt
            .and_then(|n| n.children().find(|c| c.has_tag_name("name")))
            .and_then(|n| n.text())
            .or_else(|| result.children().find(|n| n.has_tag_name("name")).and_then(|n| n.text()))
            .unwrap_or("Unnamed OpenVAS Finding")
            .to_string();

        let severity: f64 = result
            .children()
            .find(|n| n.has_tag_name("severity"))
            .and_then(|n| n.text())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let Some(our_severity) = openvas_severity_to_ours(severity) else {
            continue;
        };

        let cve = nvt
            .and_then(|n| n.children().find(|c| c.has_tag_name("cve")))
            .and_then(|n| n.text())
            .filter(|s| !s.is_empty() && *s != "NOCVE")
            .map(String::from);

        let description = result
            .children()
            .find(|n| n.has_tag_name("description"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string();

        let key = (plugin_name.clone(), our_severity.to_string());
        let finding_entry = findings.entry(key).or_insert_with(|| ParsedFinding {
            title: plugin_name,
            severity: Some(our_severity.to_string()),
            description_md: description,
            remediation_md: String::new(),
            cve,
            cvss_score: Some(severity),
            host_labels: Vec::new(),
        });
        if !finding_entry.host_labels.contains(&ip) {
            finding_entry.host_labels.push(ip.clone());
        }
    }

    let hosts = host_order
        .into_iter()
        .filter_map(|ip| hosts_by_ip.remove(&ip))
        .collect();

    Ok(ParsedImport {
        hosts,
        findings: findings.into_values().collect(),
        trust_relationships: Vec::new(),
    })
}
