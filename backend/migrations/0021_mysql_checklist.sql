-- `mysql` was in VALID_SERVICE_NAMES (services.rs) from the start but never got a
-- checklist template mapping in 0014/0016, so adding a mysql service silently
-- produced no checklist. Grounded in the operator's own notes (sql/notes.txt's
-- "FOR MYSQL" section: nmap --script mysql*, mysql -u/-p login, show databases/
-- tables/columns).
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'MySQL Enumeration', 'Standard MySQL access/enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check MySQL version for known vulnerabilities (nmap --script mysql*)",
        "Attempt default/common credential logins (mysql -u <user> -p<password> -h <ip>)",
        "Enumerate accessible databases and tables (show databases; show tables;)",
        "Search accessible tables for sensitive data/credentials",
        "Try credential reuse from other hosts/domains"
    ]
}'::jsonb
FROM t;

INSERT INTO service_checklist_templates (service_name, template_id)
SELECT 'mysql', id FROM templates WHERE kind = 'checklist' AND name = 'MySQL Enumeration';
