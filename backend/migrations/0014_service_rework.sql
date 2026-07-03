-- ADJUSTMENTS.txt #1: service logging rework. The free-text `name` field becomes a
-- controlled service-type value (validated against a fixed list in services.rs) driven
-- by a dropdown; `display_name` is a separate free-text label; `product` is dropped
-- entirely (no engagement data exists yet in this deployment, so no backfill needed).
ALTER TABLE services ADD COLUMN display_name TEXT;
ALTER TABLE services DROP COLUMN product;

-- Data-driven service-name -> checklist-template mapping, so creating a service of a
-- known type can auto-instantiate a starter checklist on that host. Extensible later
-- purely by inserting more rows here + more checklist templates -- no code change.
CREATE TABLE service_checklist_templates (
    service_name TEXT PRIMARY KEY,
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE
);

-- New starter checklist templates, one per common service type (SMB already has one
-- from 0011_seed_templates.sql, reused below rather than duplicated).
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'SSH Enumeration', 'Standard SSH access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check SSH version/banner for known vulnerabilities",
        "Attempt default/common credential logins",
        "Check for key-based auth misconfig (permitted weak algorithms)",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'FTP Enumeration', 'Standard FTP access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt anonymous login",
        "Check for writable/readable directories",
        "Attempt default/common credential logins",
        "Check for cleartext credential exposure risk (unencrypted FTP)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'RDP Enumeration', 'Standard RDP access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check NLA (Network Level Authentication) requirement",
        "Attempt default/common credential logins",
        "Check for known RDP vulnerabilities (BlueKeep etc.)",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'HTTP/HTTPS Enumeration', 'Standard web service enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Identify web server/framework and version",
        "Directory/endpoint brute-force",
        "Check for default credentials on any login pages",
        "Check TLS configuration (if HTTPS)",
        "Review for common web vulnerabilities (injection, auth bypass)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'MSSQL Enumeration', 'Standard MSSQL access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt default/common credential logins (sa etc.)",
        "Check for xp_cmdshell availability",
        "Enumerate databases and permissions",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'WinRM Enumeration', 'Standard WinRM access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt credential-based WinRM session (evil-winrm etc.)",
        "Try credential reuse from other hosts/domains",
        "Check for over-permissive WinRM access group membership"
    ]
}'::jsonb
FROM t;

INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'ssh', id FROM templates WHERE kind = 'checklist' AND name = 'SSH Enumeration'
UNION ALL
SELECT 'ftp', id FROM templates WHERE kind = 'checklist' AND name = 'FTP Enumeration'
UNION ALL
SELECT 'rdp', id FROM templates WHERE kind = 'checklist' AND name = 'RDP Enumeration'
UNION ALL
SELECT 'http', id FROM templates WHERE kind = 'checklist' AND name = 'HTTP/HTTPS Enumeration'
UNION ALL
SELECT 'https', id FROM templates WHERE kind = 'checklist' AND name = 'HTTP/HTTPS Enumeration'
UNION ALL
SELECT 'mssql', id FROM templates WHERE kind = 'checklist' AND name = 'MSSQL Enumeration'
UNION ALL
SELECT 'winrm', id FROM templates WHERE kind = 'checklist' AND name = 'WinRM Enumeration'
UNION ALL
SELECT 'smb', id FROM templates WHERE kind = 'checklist' AND name = 'SMB Enumeration';
