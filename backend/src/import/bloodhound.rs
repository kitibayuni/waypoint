use serde::Deserialize;

use super::{ParsedHost, ParsedImport, ParsedTrust};

/// Parses a BloodHound computers.json ingest file. Scope is deliberately
/// narrow: BloodHound's edge/relationship schema has changed across major
/// versions and varies by export type, so rather than guess at a fragile
/// mapping this only relies on the one shape that's been stable since
/// BloodHound 4.x -- each computer's `Properties.name` and its own
/// `Sessions.Results[].ComputerSID` list (who had a session on this
/// computer) -- and resolves sessions into trust_relationships only when
/// both ends are present in the same uploaded file.
#[derive(Deserialize)]
struct BloodHoundFile {
    data: Vec<BloodHoundComputer>,
}

#[derive(Deserialize)]
struct BloodHoundComputer {
    #[serde(rename = "ObjectIdentifier")]
    object_identifier: Option<String>,
    #[serde(rename = "Properties")]
    properties: Option<BloodHoundProperties>,
    #[serde(rename = "Sessions")]
    sessions: Option<BloodHoundEdgeList>,
}

#[derive(Deserialize)]
struct BloodHoundProperties {
    name: Option<String>,
    operatingsystem: Option<String>,
}

#[derive(Deserialize)]
struct BloodHoundEdgeList {
    #[serde(rename = "Results")]
    results: Option<Vec<BloodHoundEdge>>,
}

#[derive(Deserialize)]
struct BloodHoundEdge {
    #[serde(rename = "ComputerSID")]
    computer_sid: Option<String>,
}

pub fn parse(json: &str) -> Result<ParsedImport, String> {
    let file: BloodHoundFile = serde_json::from_str(json).map_err(|e| e.to_string())?;

    let mut hosts = Vec::new();
    // sid -> label, so Sessions edges (which reference SIDs) can be
    // resolved to the ParsedHost labels trust_relationships need.
    let mut sid_to_label = std::collections::HashMap::new();
    let mut pending_sessions: Vec<(String, String)> = Vec::new(); // (from_sid, to_sid)

    for computer in &file.data {
        let Some(name) = computer.properties.as_ref().and_then(|p| p.name.clone()) else {
            continue;
        };
        // BloodHound names are typically FQDN.DOMAIN.TLD; use the short
        // hostname as the label, matching how the rest of the app labels
        // hosts.
        let label = name.split('.').next().unwrap_or(&name).to_string();
        let os = computer.properties.as_ref().and_then(|p| p.operatingsystem.clone());

        if let Some(sid) = &computer.object_identifier {
            sid_to_label.insert(sid.clone(), label.clone());
        }

        if let (Some(to_sid), Some(sessions)) = (&computer.object_identifier, &computer.sessions)
            && let Some(results) = &sessions.results
        {
            for edge in results {
                if let Some(from_sid) = &edge.computer_sid {
                    pending_sessions.push((from_sid.clone(), to_sid.clone()));
                }
            }
        }

        hosts.push(ParsedHost {
            label,
            hostname: Some(name),
            os,
            addresses: Vec::new(),
            services: Vec::new(),
        });
    }

    let trust_relationships = pending_sessions
        .into_iter()
        .filter_map(|(from_sid, to_sid)| {
            let from_label = sid_to_label.get(&from_sid)?.clone();
            let to_label = sid_to_label.get(&to_sid)?.clone();
            Some(ParsedTrust {
                from_label,
                to_label,
                kind: "session".to_string(),
            })
        })
        .collect();

    Ok(ParsedImport {
        hosts,
        findings: Vec::new(),
        trust_relationships,
    })
}
