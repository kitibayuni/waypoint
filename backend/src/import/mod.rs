pub mod bloodhound;
pub mod nessus;
pub mod nmap;
pub mod openvas;

/// Every importer normalizes to this same shape regardless of source format,
/// so the route handler's matching/dedup/write logic is written once. Nmap
/// only ever populates hosts; Nessus adds findings; BloodHound adds hosts +
/// trust_relationships.
#[derive(Debug, Default)]
pub struct ParsedImport {
    pub hosts: Vec<ParsedHost>,
    pub findings: Vec<ParsedFinding>,
    pub trust_relationships: Vec<ParsedTrust>,
}

#[derive(Debug, Clone)]
pub struct ParsedHost {
    pub label: String,
    pub hostname: Option<String>,
    pub os: Option<String>,
    pub addresses: Vec<String>,
    pub services: Vec<ParsedService>,
}

#[derive(Debug, Clone)]
pub struct ParsedService {
    pub port: i32,
    pub protocol: String,
    pub name: Option<String>,
    pub product: Option<String>,
    pub version: Option<String>,
}

/// `host_labels` references `ParsedHost::label` values from the same
/// `ParsedImport`, resolved to real host ids by the route handler after
/// hosts are created/matched.
#[derive(Debug, Clone)]
pub struct ParsedFinding {
    pub title: String,
    pub severity: Option<String>,
    pub description_md: String,
    pub remediation_md: String,
    pub cve: Option<String>,
    pub cvss_score: Option<f64>,
    pub host_labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedTrust {
    pub from_label: String,
    pub to_label: String,
    pub kind: String,
}
