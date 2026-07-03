-- MITRE ATT&CK reference table. Existing observation_types.mitre_technique_ids
-- (jsonb array) and attack_path_rules.mitre_technique_id (text) already store
-- technique IDs; this is a lookup table joined by matching those ID strings
-- for display (human-readable names), not a literal FK (a jsonb array can't
-- carry one).
CREATE TABLE mitre_techniques (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    tactic TEXT,
    url TEXT
);

INSERT INTO mitre_techniques (id, name, tactic, url) VALUES
('T1557', 'Adversary-in-the-Middle', 'Credential Access, Collection', 'https://attack.mitre.org/techniques/T1557/'),
('T1557.001', 'LLMNR/NBT-NS Poisoning and SMB Relay', 'Credential Access, Collection', 'https://attack.mitre.org/techniques/T1557/001/'),
('T1558.003', 'Kerberoasting', 'Credential Access', 'https://attack.mitre.org/techniques/T1558/003/'),
('T1087.002', 'Domain Account Discovery', 'Discovery', 'https://attack.mitre.org/techniques/T1087/002/'),
('T1078', 'Valid Accounts', 'Defense Evasion, Persistence, Privilege Escalation, Initial Access', 'https://attack.mitre.org/techniques/T1078/');

-- DESIGN.md §9 lists mitre_technique_ids on findings alongside
-- observation_types/attack_path_rules, but the original findings migration
-- (0005) didn't include it.
ALTER TABLE findings ADD COLUMN mitre_technique_ids JSONB NOT NULL DEFAULT '[]';
