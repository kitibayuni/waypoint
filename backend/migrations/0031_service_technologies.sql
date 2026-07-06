-- Layers a detected application/technology on top of a protocol-level
-- service (e.g. an http/https service that's specifically running
-- WordPress or Jenkins), distinct from the service's own protocol-level
-- checklist -- common-apps/ documents per-application enumeration
-- playbooks (WordPress, Drupal, Joomla, Jenkins, GitLab, Tomcat,
-- ColdFusion, osTicket, PRTG, Splunk) that a generic "HTTP/HTTPS
-- Enumeration" checklist doesn't cover.
--
-- technology_checklist_templates mirrors service_checklist_templates
-- exactly (same PK-on-name-string shape) rather than reusing that table,
-- since "http" the protocol and "wordpress" the technology are different
-- keying namespaces that could otherwise collide.
CREATE TABLE service_technologies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    version TEXT,
    notes_md TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_service_technologies_service_id ON service_technologies(service_id);

CREATE TABLE technology_checklist_templates (
    technology_name TEXT PRIMARY KEY,
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE
);

-- Only the two best-documented technologies in common-apps/ get a starter
-- checklist for now; more can be added the same way without another
-- migration (service_checklist_templates already grew incrementally the
-- same way, e.g. 0021_mysql_checklist.sql).
WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'WordPress Enumeration', 'Standard WordPress enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Enumerate version, theme, and plugins (wpscan --url <target> --enumerate vp,vt)",
        "Check for known CVEs against identified plugin/theme versions",
        "Check for user enumeration via /wp-json/wp/v2/users or ?author=1",
        "Attempt login brute-force/default creds against /wp-login.php (rate-limit aware)",
        "Check for exposed xmlrpc.php (pingback amplification / brute-force bypass)"
    ]
}'::jsonb
FROM t;
INSERT INTO technology_checklist_templates (technology_name, template_id)
SELECT 'wordpress', id FROM templates WHERE kind = 'checklist' AND name = 'WordPress Enumeration';

WITH t AS (
    INSERT INTO templates (kind, name, description, is_shared)
    VALUES ('checklist', 'Jenkins Enumeration', 'Standard Jenkins enumeration steps.', TRUE)
    RETURNING id
)
INSERT INTO template_payloads (template_id, body)
SELECT id, '{
    "items": [
        "Check for unauthenticated access to /script (Groovy console = RCE)",
        "Check for anonymous read/build permissions on jobs",
        "Enumerate stored credentials via the Script Console if RCE is achieved",
        "Check version against known Jenkins CVEs",
        "Check for exposed build logs/artifacts containing embedded secrets"
    ]
}'::jsonb
FROM t;
INSERT INTO technology_checklist_templates (technology_name, template_id)
SELECT 'jenkins', id FROM templates WHERE kind = 'checklist' AND name = 'Jenkins Enumeration';
