-- Seed a starter template set per DESIGN.md §12: an "AD Host Recon" host
-- template (checklist synthesized from active-directory/initial-recon.txt
-- and the LLMNR/SMB notes) and a "Login Procedure" note template from
-- myweaknesses.txt's explicit "need to build templates for 'login
-- procedure'" gap. One example each for checklist/finding/engagement
-- kinds too, so every template kind has a working seed.

-- host: AD Host Recon
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('host', 'AD Host Recon',
        'A Windows/AD host with a starter enumeration checklist and login-procedure note attached.',
        TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "tags": ["domain-joined"],
    "checklists": [
        {
            "name": "AD Host Recon",
            "items": [
                "Enumerate AD users (Get-NetUser / ldapsearch)",
                "Enumerate AD joined computers",
                "Identify key services running on this host",
                "Check for known vulnerable hosts/services",
                "Check password policy",
                "Attempt anonymous LDAP bind",
                "Check for LLMNR/NBT-NS poisoning opportunity (Responder)",
                "Check SMB signing status"
            ]
        }
    ],
    "notes": [
        {
            "title": "Login Procedure",
            "body_md": "**Access methods attempted:**\n- [ ] RDP\n- [ ] WinRM\n- [ ] SSH\n- [ ] SMB / PsExec\n- [ ] Web login\n\n**Credentials tried:**\n- \n\n**Notes:**\n- Remember to try DOMAIN\\username format for AD logins.\n- Try credential reuse from other hosts/domains before brute-forcing.\n- If using your own account, check whether it has more privileges than the target default creds."
        }
    ]
}'::jsonb
FROM t;

-- checklist: SMB Enumeration (standalone, synthesized from common-services/smb.txt)
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'SMB Enumeration', 'Standard SMB share/signing enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "List available shares (smbclient -L or crackmapexec)",
        "Check SMB signing requirement",
        "Attempt anonymous/null session",
        "Mount and search shares for credentials (findstr / grep for cred, secret, passw)",
        "Check for writable shares"
    ]
}'::jsonb
FROM t;

-- note: Login Procedure (standalone, reusable outside a host template)
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('note', 'Login Procedure', 'Checklist-style note for tracking access attempts against a target.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "title": "Login Procedure",
    "body_md": "**Access methods attempted:**\n- [ ] RDP\n- [ ] WinRM\n- [ ] SSH\n- [ ] SMB / PsExec\n- [ ] Web login\n\n**Credentials tried:**\n- \n\n**Notes:**\n- Remember to try DOMAIN\\username format for AD logins.\n- Try credential reuse from other hosts/domains before brute-forcing."
}'::jsonb
FROM t;

-- finding: Weak TLS Configuration (scaffold, ties back to the observation type)
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('finding', 'Weak TLS Configuration', 'Prefilled finding scaffold for weak TLS/SSL configurations.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "title": "Weak TLS Configuration",
    "severity": "medium",
    "description_md": "The service supports outdated TLS/SSL protocol versions or weak cipher suites, exposing it to protocol-downgrade or cryptographic attacks.",
    "remediation_md": "Disable SSLv3/TLS 1.0/TLS 1.1 and weak cipher suites; require TLS 1.2+ with strong ciphers only.",
    "poc_md": "",
    "references_json": ["https://ssl-config.mozilla.org/"]
}'::jsonb
FROM t;

-- engagement: Standard Pentest Engagement (kickoff scaffold)
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('engagement', 'Standard Pentest Engagement',
        'Kickoff scaffold following the 8-step vulnerability assessment methodology.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "standard_findings": [
        {
            "title": "Executive Summary Placeholder",
            "severity": "info",
            "description_md": "Summarize the engagement scope, methodology, and highest-priority findings here before delivering the report.",
            "remediation_md": "",
            "poc_md": "",
            "references_json": []
        }
    ]
}'::jsonb
FROM t;
