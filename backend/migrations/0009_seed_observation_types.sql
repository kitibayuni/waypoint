-- Seed the observation-type catalog from the notes in DESIGN.md §12.
-- Editable afterward via the observation_types API; this just gives a
-- fresh install a useful starting catalog rather than an empty one.

INSERT INTO observation_types (key, title, category, default_severity, description_md, references_json) VALUES
('smb_signing_disabled', 'SMB Signing Disabled', 'smb', 'medium',
 'SMB signing is not required on this host, allowing NTLM relay attacks against SMB.',
 '["https://attack.mitre.org/techniques/T1557/001/"]'),
('llmnr_enabled', 'LLMNR/NBT-NS Enabled', 'ad', 'medium',
 'Link-Local Multicast Name Resolution and NetBIOS Name Service are fallback name-resolution methods used when DNS fails. An attacker on the same broadcast segment can poison these responses (e.g. via Responder or Inveigh) and capture NTLMv2 hashes.',
 '["https://attack.mitre.org/techniques/T1557/001/"]'),
('ldap_anonymous_bind', 'LDAP Anonymous Bind', 'ad', 'medium',
 'The LDAP service accepts anonymous binds, allowing unauthenticated enumeration of directory contents (users, groups, computers, policies).',
 '[]'),
('default_credentials', 'Default Credentials', 'creds', 'high',
 'A service or application is accessible using vendor-default or otherwise well-known credentials.',
 '[]'),
('weak_tls_config', 'Weak TLS Configuration', 'tls', 'medium',
 'The service supports outdated TLS/SSL protocol versions or weak cipher suites, exposing it to protocol-downgrade or cryptographic attacks.',
 '[]'),
('kerberoastable_spn', 'Kerberoastable SPN', 'ad', 'high',
 'A user account has a Service Principal Name registered, allowing any authenticated domain user to request a TGS ticket for it and attempt offline cracking of the service account''s password (Kerberoasting).',
 '["https://attack.mitre.org/techniques/T1558/003/"]')
ON CONFLICT (key) DO NOTHING;
