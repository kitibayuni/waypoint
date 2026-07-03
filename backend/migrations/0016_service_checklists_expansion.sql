-- ADJUSTMENTS.txt follow-up: give every service in the dropdown its own checklist
-- section, and ground every item (existing and new) in the operator's actual
-- technique notes rather than generic textbook steps.

-- Enrich the 7 checklists seeded in 0014 with note-derived specifics.
UPDATE template_payloads SET body = '{
    "items": [
        "Run ssh-audit against the target and check version/banner for known CVEs (ssh-audit.py <target-ip>)",
        "Force password authentication and test manually (ssh -o PreferredAuthentications=password)",
        "Attempt default/common credential logins",
        "Brute-force with hydra if authorized (hydra -L users.txt -P passwords.txt -t 4 -V ssh://target)",
        "Check for key-based auth misconfig (permitted weak algorithms)",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
WHERE template_id = (SELECT id FROM templates WHERE kind = 'checklist' AND name = 'SSH Enumeration');

UPDATE template_payloads SET body = '{
    "items": [
        "Attempt anonymous login (username anonymous, blank password)",
        "Recursively list shares and pull suspicious files for offline review (ls -R, get/mget)",
        "Brute-force with medusa if authorized (medusa -u user -P rockyou.txt -h target -M ftp -n port)",
        "Check for FTP bounce potential (nmap -b anonymous:pass@intermediate-ip target)",
        "Toggle passive mode if directory listing hangs",
        "Check for known-vulnerable FTP daemon versions (e.g. CoreFTP path-traversal PoC)"
    ]
}'::jsonb
WHERE template_id = (SELECT id FROM templates WHERE kind = 'checklist' AND name = 'FTP Enumeration');

UPDATE template_payloads SET body = '{
    "items": [
        "Run rdp-sec-check / nmap --script rdp* to assess protocol security and NLA requirement",
        "Attempt default/common credential logins (xfreerdp /u:administrator /p:administrator /cert:ignore)",
        "Brute-force with crowbar/hydra if authorized",
        "If a captured hash is available, attempt pass-the-hash (clear DisableRestrictedAdmin first, then xfreerdp /pth:<hash>)",
        "If later given a shell, check for session hijack potential (query user, tscon)"
    ]
}'::jsonb
WHERE template_id = (SELECT id FROM templates WHERE kind = 'checklist' AND name = 'RDP Enumeration');

UPDATE template_payloads SET body = '{
    "items": [
        "Fingerprint server/framework/tech stack (whatweb, Wappalyzer, nmap -sV, curl -I)",
        "Check robots.txt, security.txt, and other well-known URIs for hidden paths",
        "Directory/vhost brute-force (gobuster/ffuf; gobuster vhost --append-domain --domain target)",
        "Test basic-auth and login forms for weak/default creds (hydra http-get / http-post-form)",
        "Run nikto for a quick software-identification pass (nikto -h target -Tuning b)",
        "Check TLS configuration (if HTTPS)"
    ]
}'::jsonb
WHERE template_id = (SELECT id FROM templates WHERE kind = 'checklist' AND name = 'HTTP/HTTPS Enumeration');

UPDATE template_payloads SET body = '{
    "items": [
        "Attempt default/common credential logins (sa / blank, mssqlclient.py, sqlcmd)",
        "Check current privileges and IMPERSONATE grants (enum_impersonate / sys.server_permissions)",
        "Check for xp_cmdshell availability; enable via sp_configure if sysadmin",
        "Enumerate linked servers and try command execution through them (EXECUTE ... AT [server])",
        "Attempt hash theft via xp_dirtree/xp_subdirs against a Responder or impacket-smbserver listener",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
WHERE template_id = (SELECT id FROM templates WHERE kind = 'checklist' AND name = 'MSSQL Enumeration');

UPDATE template_payloads SET body = '{
    "items": [
        "Attempt credential-based WinRM session (evil-winrm -i target -u user -p pass)",
        "Brute-force with netexec/hydra if authorized (netexec winrm target -u users.list -p passwords.list)",
        "Try credential reuse from other hosts/domains",
        "Check for over-permissive WinRM access group membership",
        "If SQL Server Management Studio tooling is reachable, enumerate databases via Invoke-Sqlcmd"
    ]
}'::jsonb
WHERE template_id = (SELECT id FROM templates WHERE kind = 'checklist' AND name = 'WinRM Enumeration');

UPDATE template_payloads SET body = '{
    "items": [
        "List available shares anonymously (smbclient -N -L //target, smbmap -H target)",
        "Check SMB signing requirement and NTLM relay potential (Responder + impacket-ntlmrelayx)",
        "Attempt anonymous/null RPC session (rpcclient -U \"\" target, enumdomusers)",
        "Mount/browse shares and grep for credential-shaped filenames/content (findstr or Select-String for cred, secret, passw)",
        "Brute-force with crackmapexec/netexec if authorized",
        "Check for PsExec-style RCE and hash dumping (crackmapexec --sam / --exec-method smbexec, impacket-psexec)"
    ]
}'::jsonb
WHERE template_id = (SELECT id FROM templates WHERE kind = 'checklist' AND name = 'SMB Enumeration');

-- New per-service checklist templates, one per remaining dropdown value.
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Telnet Enumeration', 'Standard Telnet access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt default/common credential logins (root:root, admin:admin)",
        "Grab the banner and identify service/version (nc -nv target port)",
        "Brute-force with hydra/netexec if authorized"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'VNC Enumeration', 'Standard VNC access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt connection with no/blank/default password (vncviewer target)",
        "Brute-force with hydra/netexec if authorized",
        "Check VNC auth version/weak-auth support (nmap -sV --script vnc-info,realvnc-auth-bypass)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'PostgreSQL Enumeration', 'Standard PostgreSQL access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt default/common credential logins (psql -h target -U postgres)",
        "List databases and tables once connected (\\\\l, \\\\dt, or information_schema equivalents)",
        "Check for superuser/role misconfiguration",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'LDAP Enumeration', 'Standard LDAP access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt anonymous/unauthenticated bind (ldapsearch -x -H ldap://target -b \"dc=...,dc=...\")",
        "Enumerate users/groups/computers with valid or anonymous creds",
        "Check application logic for LDAP injection (inject * into username/password fields)",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'DNS Enumeration', 'Standard DNS access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Attempt a zone transfer (dig axfr @nameserver zone)",
        "Pull all records and enumerate subdomains (dig any, dnsenum, subfinder/subbrute)",
        "Check zone config for risky settings if BIND files are reachable (allow-transfer, allow-recursion)",
        "Check for subdomain/domain takeover candidates (dangling CNAMEs)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'SNMP Enumeration', 'Standard SNMP access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Brute-force the community string (onesixtyone -c wordlist.txt target)",
        "Walk the MIB tree once a community string is found (snmpwalk -v2c -c <string> target)",
        "Brute-force individual OIDs if needed (braa <string>@target:.1.3.4.*)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'NFS Enumeration', 'Standard NFS access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "List exported shares (showmount -e target)",
        "Mount an exported share and browse for sensitive files (mount -t nfs target:/share ./mnt -o nolock)",
        "Check export permissions for overly-permissive (everyone / no_root_squash) shares"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'SMTP Enumeration', 'Standard SMTP access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Enumerate valid users via VRFY/EXPN/RCPT TO (smtp-user-enum or manual telnet)",
        "Check for open relay (nmap -p25 --script smtp-open-relay)",
        "Check MX records and resolve the mail server IP (host -t MX, dig mx)",
        "If an open relay is found, test sending a message through it (swaks)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'POP3 Enumeration', 'Standard POP3 access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Enumerate users via the USER command over telnet/openssl",
        "Brute-force credentials if authorized (hydra -L users -P passwords pop3; use the full username@domain if seen literally)",
        "Connect encrypted and list the mailbox (openssl s_client -connect target:pop3s, then USER/PASS/STAT)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'IMAP Enumeration', 'Standard IMAP access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Connect encrypted and authenticate (openssl s_client -connect target:imaps, then a login <user> <pass>)",
        "List and search mailboxes once authenticated (a list \"\" *, a select <mailbox>, a search all)",
        "Fetch messages carefully -- a full fetch may mark them read (use BODY.PEEK[...] instead)",
        "Brute-force credentials if authorized"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Rsync Enumeration', 'Standard rsync access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Probe for an accessible module/share (nc -nv target port)",
        "List an open module''s contents without transferring (rsync -av --list-only rsync://target/share)",
        "Check for anonymous read/write access on any discovered module"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Oracle TNS Enumeration', 'Standard Oracle TNS access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Enumerate the SID (nmap -p<port> --script oracle-sid-brute)",
        "Run odat against the target for a full automated sweep (odat.py all -s target)",
        "Attempt default credentials (CHANGE_ON_INSTALL for Oracle 9, dbsnmp/dbsnmp)",
        "If logged in, check table/role privileges and attempt password hash extraction (select name, password from sys.user$)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'IPMI Enumeration', 'Standard IPMI access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Footprint the service version (nmap -sU --script ipmi-version -p 623)",
        "Attempt hash extraction via the IPMI 2.0 RAKP vulnerability (msf auxiliary/scanner/ipmi/ipmi_dumphashes)",
        "Crack extracted hashes offline (hashcat -m 7300)"
    ]
}'::jsonb
FROM t;

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'R-Services Enumeration', 'Standard rsh/rlogin/rexec access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for trusted-host misconfiguration (/etc/hosts.equiv, ~/.rhosts)",
        "Attempt rlogin with a plausible username (rlogin target -l user)",
        "Enumerate logged-in/authenticated users (rwho, rusers -al target)"
    ]
}'::jsonb
FROM t;

-- Map every dropdown service (except the "other" catch-all) to a checklist template.
INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'telnet', id FROM templates WHERE kind = 'checklist' AND name = 'Telnet Enumeration'
UNION ALL
SELECT 'vnc', id FROM templates WHERE kind = 'checklist' AND name = 'VNC Enumeration'
UNION ALL
SELECT 'postgresql', id FROM templates WHERE kind = 'checklist' AND name = 'PostgreSQL Enumeration'
UNION ALL
SELECT 'ldap', id FROM templates WHERE kind = 'checklist' AND name = 'LDAP Enumeration'
UNION ALL
SELECT 'dns', id FROM templates WHERE kind = 'checklist' AND name = 'DNS Enumeration'
UNION ALL
SELECT 'snmp', id FROM templates WHERE kind = 'checklist' AND name = 'SNMP Enumeration'
UNION ALL
SELECT 'nfs', id FROM templates WHERE kind = 'checklist' AND name = 'NFS Enumeration'
UNION ALL
SELECT 'smtp', id FROM templates WHERE kind = 'checklist' AND name = 'SMTP Enumeration'
UNION ALL
SELECT 'pop3', id FROM templates WHERE kind = 'checklist' AND name = 'POP3 Enumeration'
UNION ALL
SELECT 'imap', id FROM templates WHERE kind = 'checklist' AND name = 'IMAP Enumeration'
UNION ALL
SELECT 'rsync', id FROM templates WHERE kind = 'checklist' AND name = 'Rsync Enumeration'
UNION ALL
SELECT 'oracle', id FROM templates WHERE kind = 'checklist' AND name = 'Oracle TNS Enumeration'
UNION ALL
SELECT 'ipmi', id FROM templates WHERE kind = 'checklist' AND name = 'IPMI Enumeration'
UNION ALL
SELECT 'rsh', id FROM templates WHERE kind = 'checklist' AND name = 'R-Services Enumeration';
