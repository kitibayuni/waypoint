use super::{ParsedHost, ParsedImport, ParsedService};

/// Parses `nmap -oX` output. Hosts reported as down are skipped; only ports
/// in the "open" state become services.
pub fn parse(xml: &str) -> Result<ParsedImport, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;

    let mut hosts = Vec::new();

    for host_node in doc.descendants().filter(|n| n.has_tag_name("host")) {
        let is_up = host_node
            .children()
            .find(|n| n.has_tag_name("status"))
            .and_then(|n| n.attribute("state"))
            .map(|s| s == "up")
            .unwrap_or(true);
        if !is_up {
            continue;
        }

        let addresses: Vec<String> = host_node
            .children()
            .filter(|n| n.has_tag_name("address"))
            .filter(|n| matches!(n.attribute("addrtype"), Some("ipv4") | Some("ipv6")))
            .filter_map(|n| n.attribute("addr").map(String::from))
            .collect();
        if addresses.is_empty() {
            continue;
        }

        let hostname = host_node
            .children()
            .find(|n| n.has_tag_name("hostnames"))
            .and_then(|hn| hn.children().find(|n| n.has_tag_name("hostname")))
            .and_then(|n| n.attribute("name"))
            .map(String::from);

        let os = host_node
            .children()
            .find(|n| n.has_tag_name("os"))
            .and_then(|os_node| os_node.children().find(|n| n.has_tag_name("osmatch")))
            .and_then(|n| n.attribute("name"))
            .map(String::from);

        let mut services = Vec::new();
        if let Some(ports_node) = host_node.children().find(|n| n.has_tag_name("ports")) {
            for port_node in ports_node.children().filter(|n| n.has_tag_name("port")) {
                let is_open = port_node
                    .children()
                    .find(|n| n.has_tag_name("state"))
                    .and_then(|n| n.attribute("state"))
                    .map(|s| s == "open")
                    .unwrap_or(false);
                if !is_open {
                    continue;
                }

                let port: i32 = port_node
                    .attribute("portid")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let protocol = port_node.attribute("protocol").unwrap_or("tcp").to_string();

                let service_node = port_node.children().find(|n| n.has_tag_name("service"));
                let name = service_node.and_then(|n| n.attribute("name")).map(String::from);
                let product = service_node.and_then(|n| n.attribute("product")).map(String::from);
                let version = service_node.and_then(|n| n.attribute("version")).map(String::from);

                services.push(ParsedService {
                    port,
                    protocol,
                    name,
                    product,
                    version,
                });
            }
        }

        let label = hostname.clone().unwrap_or_else(|| addresses[0].clone());

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
        ..Default::default()
    })
}
