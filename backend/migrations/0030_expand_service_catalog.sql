-- Adds checklist templates for 13 services referenced by name in
-- enterprise-networks/2.service-enum-n-exploit.txt with no prior mapping
-- (mostly modern/cloud-adjacent services absent from the original
-- protocol-era catalog). Same seed shape as 0021_mysql_checklist.sql.
-- VALID_SERVICE_NAMES (services.rs) and SERVICE_NAMES (services.ts) are
-- extended to match in the same commit as this migration.

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Redis Enumeration', 'Standard Redis access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt unauthenticated connection (redis-cli -h <target>)",
        "Try common/default passwords (redis-cli -h <target> -a <password>)",
        "If accessible, dump keys and inspect for credentials (KEYS *, GET <key>)",
        "Check for the CONFIG SET dir/dbfilename webshell write technique",
        "Check for master/replica misconfig exposing replication data"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'redis', id FROM templates WHERE kind = 'checklist' AND name = 'Redis Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'MongoDB Enumeration', 'Standard MongoDB access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt unauthenticated connection (mongosh mongodb://<target>:27017)",
        "Try common/default credentials (mongosh mongodb://admin:admin@<target>:27017)",
        "Enumerate databases and collections (show dbs, use <db>, show collections)",
        "Search accessible collections for sensitive data/credentials",
        "Check bound-IP/auth configuration for external exposure"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'mongodb', id FROM templates WHERE kind = 'checklist' AND name = 'MongoDB Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Elasticsearch Enumeration', 'Standard Elasticsearch access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for unauthenticated access (curl http://<target>:9200/)",
        "Enumerate indices (curl http://<target>:9200/_cat/indices)",
        "Try common/default credentials (curl -u elastic:changeme http://<target>:9200/)",
        "Search accessible indices for sensitive data/credentials",
        "Check version against known Elasticsearch/Kibana CVEs"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'elasticsearch', id FROM templates WHERE kind = 'checklist' AND name = 'Elasticsearch Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Cassandra Enumeration', 'Standard Cassandra access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt unauthenticated connection (cqlsh <target>)",
        "Try default credentials (cqlsh <target> -u cassandra -p cassandra)",
        "Enumerate keyspaces and tables (DESCRIBE keyspaces; DESCRIBE tables;)",
        "Search accessible tables for sensitive data/credentials"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'cassandra', id FROM templates WHERE kind = 'checklist' AND name = 'Cassandra Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Memcached Enumeration', 'Standard Memcached access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for unauthenticated access (echo \"stats\" | nc <target> 11211)",
        "Enumerate cached item keys (echo \"stats items\" | nc <target> 11211)",
        "Dump cache contents for a given slab (echo \"stats cachedump 1 0\" | nc <target> 11211)",
        "Search dumped cache contents for sensitive data/credentials"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'memcached', id FROM templates WHERE kind = 'checklist' AND name = 'Memcached Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Docker API Enumeration', 'Standard exposed Docker daemon API enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for unauthenticated access (docker -H tcp://<target>:2375 ps)",
        "Enumerate images (docker -H tcp://<target>:2375 images) or curl http://<target>:2375/containers/json",
        "If writable, spin up a container with a mounted host path for a container-escape foothold",
        "Check for TLS client-cert auth (port 2376) vs. plaintext (2375)"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'docker_api', id FROM templates WHERE kind = 'checklist' AND name = 'Docker API Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Kubernetes API Enumeration', 'Standard exposed Kubernetes API server enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for unauthenticated access (curl -k https://<target>:6443/api/v1/namespaces)",
        "Check the read-only kubelet API if exposed (curl http://<target>:10255/pods or -k https://<target>:10250/pods)",
        "Check for anonymous-auth enabled or a leaked service account token",
        "If accessible, enumerate secrets (kubectl get secrets --all-namespaces) for embedded credentials"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'kubernetes_api', id FROM templates WHERE kind = 'checklist' AND name = 'Kubernetes API Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'MQTT Enumeration', 'Standard MQTT broker enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for unauthenticated access (mosquitto_sub -h <target> -t \"#\" -v)",
        "Try default credentials (mosquitto_sub -h <target> -t \"#\" -u admin -P admin -v)",
        "Monitor traffic for sensitive data in topic payloads",
        "Check whether publishing to arbitrary topics is allowed (mosquitto_pub)"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'mqtt', id FROM templates WHERE kind = 'checklist' AND name = 'MQTT Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'SIP Enumeration', 'Standard SIP/VoIP enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Map SIP services on the target (svmap <target>)",
        "Enumerate valid extensions (sipvicious svwar -e 100-999 <target>)",
        "Attempt password cracking against a discovered extension (svcrack -u <ext> -d <wordlist> <target>)",
        "Check for toll fraud / unauthorized call routing exposure"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'sip', id FROM templates WHERE kind = 'checklist' AND name = 'SIP Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'RTSP Enumeration', 'Standard RTSP (camera/streaming) enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for unauthenticated stream access (ffplay rtsp://<target>:554/)",
        "Try default credentials (ffplay rtsp://admin:admin@<target>:554/)",
        "Enumerate common stream paths for the device vendor",
        "Check for known CVEs against the identified camera/NVR firmware"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'rtsp', id FROM templates WHERE kind = 'checklist' AND name = 'RTSP Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'AJP Enumeration', 'Standard Apache JServ Protocol (Tomcat) enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Scan for AJP auth/request issues (nmap --script ajp-auth,ajp-request -p 8009 <target>)",
        "Check for the Ghostcat file-read/include vulnerability (CVE-2020-1938) if an older Tomcat version",
        "Confirm whether AJP is reachable externally vs. loopback-only (should not be internet-facing)"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'ajp', id FROM templates WHERE kind = 'checklist' AND name = 'AJP Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'TFTP Enumeration', 'Standard TFTP enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Connect and probe for readable files (tftp <target>, then get <filename>)",
        "Try common config/firmware filenames for network devices (running-config, startup-config, .cfg)",
        "Search retrieved files for embedded credentials/secrets",
        "Check whether writes are also permitted (put), not just reads"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'tftp', id FROM templates WHERE kind = 'checklist' AND name = 'TFTP Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'LDAPS Enumeration', 'Standard LDAP-over-TLS enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt anonymous bind over TLS (ldapsearch -x -H ldaps://<target> -b \"dc=domain,dc=local\")",
        "Try default/empty credentials against a known service account",
        "Enumerate users/groups/computers once bound (same queries as plain LDAP)",
        "Confirm certificate validation is not silently bypassed by tooling in a way that masks a MITM"
    ]
}'::jsonb
FROM t;
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'ldaps', id FROM templates WHERE kind = 'checklist' AND name = 'LDAPS Enumeration';
