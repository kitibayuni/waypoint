use std::collections::HashMap;

use super::{ParsedFinding, ParsedHost, ParsedImport, ParsedService};

fn nessus_severity_to_ours(severity: &str) -> Option<&'static str> {
    match severity {
        "1" => Some("low"),
        "2" => Some("medium"),
        "3" => Some("high"),
        "4" => Some("critical"),
        // severity 0 is informational (e.g. "port is open") -- not a
        // finding, just service-discovery data already captured above.
        _ => None,
    }
}

/// Parses a `.nessus` (NessusClientData_v2) export. ReportItems with
/// severity 0 only contribute port/service data; severity >= 1 items become
/// findings, grouped by (plugin name, severity) across all hosts in the
/// file so one vulnerability type produces one finding with every affected
/// host attached, rather than a duplicate finding per host.
pub fn parse(xml: &str) -> Result<ParsedImport, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;

    let mut hosts = Vec::new();
    // Keyed by (plugin_name, severity) to merge the same vulnerability
    // across hosts into a single finding.
    let mut findings: HashMap<(String, String), ParsedFinding> = HashMap::new();

    for report_host in doc.descendants().filter(|n| n.has_tag_name("ReportHost")) {
        let report_host_name = report_host.attribute("name").unwrap_or("").to_string();

        let mut ip = None;
        let mut hostname = None;
        let mut os = None;
        if let Some(props) = report_host.children().find(|n| n.has_tag_name("HostProperties")) {
            for tag in props.children().filter(|n| n.has_tag_name("tag")) {
                let name = tag.attribute("name").unwrap_or("");
                let value = tag.text().unwrap_or("").to_string();
                match name {
                    "host-ip" => ip = Some(value),
                    "host-fqdn" => hostname = Some(value),
                    "operating-system" => os = Some(value),
                    _ => {}
                }
            }
        }

        let mut addresses = Vec::new();
        if let Some(ip) = &ip {
            addresses.push(ip.clone());
        } else if report_host_name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            // ReportHost's own `name` attribute is often the IP itself.
            addresses.push(report_host_name.clone());
        }
        if addresses.is_empty() {
            continue;
        }

        let label = hostname
            .clone()
            .unwrap_or_else(|| report_host_name.clone())
            .to_string();
        let label = if label.is_empty() { addresses[0].clone() } else { label };

        let mut services: Vec<ParsedService> = Vec::new();
        let mut seen_ports = std::collections::HashSet::new();

        for item in report_host.children().filter(|n| n.has_tag_name("ReportItem")) {
            let port: i32 = item.attribute("port").and_then(|s| s.parse().ok()).unwrap_or(0);
            let protocol = item.attribute("protocol").unwrap_or("tcp").to_string();
            let svc_name = item.attribute("svc_name").map(String::from);

            if port > 0 && seen_ports.insert((port, protocol.clone())) {
                services.push(ParsedService {
                    port,
                    protocol: protocol.clone(),
                    name: svc_name,
                    product: None,
                    version: None,
                });
            }

            let severity = item.attribute("severity").unwrap_or("0");
            let Some(our_severity) = nessus_severity_to_ours(severity) else {
                continue;
            };

            let plugin_name = item
                .attribute("pluginName")
                .unwrap_or("Unnamed Nessus Finding")
                .to_string();

            let description = item
                .children()
                .find(|n| n.has_tag_name("description"))
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string();
            let solution = item
                .children()
                .find(|n| n.has_tag_name("solution"))
                .and_then(|n| n.text())
                .unwrap_or("")
                .to_string();
            let cve = item
                .children()
                .find(|n| n.has_tag_name("cve"))
                .and_then(|n| n.text())
                .map(String::from);
            let cvss_score = item
                .children()
                .find(|n| n.has_tag_name("cvss_base_score"))
                .and_then(|n| n.text())
                .and_then(|s| s.parse().ok());

            let key = (plugin_name.clone(), our_severity.to_string());
            let entry = findings.entry(key).or_insert_with(|| ParsedFinding {
                title: plugin_name,
                severity: Some(our_severity.to_string()),
                description_md: description,
                remediation_md: solution,
                cve,
                cvss_score,
                host_labels: Vec::new(),
            });
            if !entry.host_labels.contains(&label) {
                entry.host_labels.push(label.clone());
            }
        }

        hosts.push(ParsedHost {
            label,
            hostname,
            os,
            addresses,
            services,
        });
    }

    Ok(ParsedImport {
        hosts,
        findings: findings.into_values().collect(),
        trust_relationships: Vec::new(),
    })
}
