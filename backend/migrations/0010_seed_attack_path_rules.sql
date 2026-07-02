-- Seed attack-path rules linking the observation-type catalog (0009) to
-- suggested exploitation techniques, per instructions.txt's worked
-- examples and DESIGN.md §12.

INSERT INTO attack_path_rules (trigger_observation_type_id, technique, outcome, next_step_md, mitre_technique_id)
SELECT id, 'NTLM Relay', 'Lateral Movement',
    'Run ntlmrelayx.py (or Responder in relay mode) to relay captured NTLM authentication to another host where SMB signing is also disabled.',
    'T1557.001'
FROM observation_types WHERE key = 'smb_signing_disabled';

INSERT INTO attack_path_rules (trigger_observation_type_id, technique, outcome, next_step_md, mitre_technique_id)
SELECT id, 'Responder', 'Hash Capture',
    'Run Responder (Linux) or Inveigh (Windows) to poison LLMNR/NBT-NS broadcasts and capture NTLMv2 hashes for offline cracking with hashcat -m 5600.',
    'T1557.001'
FROM observation_types WHERE key = 'llmnr_enabled';

INSERT INTO attack_path_rules (trigger_observation_type_id, technique, outcome, next_step_md, mitre_technique_id)
SELECT id, 'Kerberoasting', 'Offline Credential Cracking',
    'Request a TGS ticket for the service account with GetUserSPNs.py -request and crack it offline with hashcat -m 13100.',
    'T1558.003'
FROM observation_types WHERE key = 'kerberoastable_spn';

INSERT INTO attack_path_rules (trigger_observation_type_id, technique, outcome, next_step_md, mitre_technique_id)
SELECT id, 'Anonymous LDAP Enumeration', 'Domain Reconnaissance',
    'Enumerate users, groups, computers, and password/lockout policies via anonymous LDAP bind to build a target list for password spraying.',
    'T1087.002'
FROM observation_types WHERE key = 'ldap_anonymous_bind';

INSERT INTO attack_path_rules (trigger_observation_type_id, technique, outcome, next_step_md, mitre_technique_id)
SELECT id, 'Credential Reuse', 'Initial Access',
    'Authenticate with the default credentials directly, then check whether the same credentials work on other services in scope.',
    'T1078'
FROM observation_types WHERE key = 'default_credentials';

INSERT INTO attack_path_rules (trigger_observation_type_id, technique, outcome, next_step_md, mitre_technique_id)
SELECT id, 'Protocol Downgrade / MITM', 'Credential Interception',
    'Position on-path (ARP spoofing, rogue AP, or an existing foothold) and force a downgrade to the weak cipher/protocol to intercept credentials in transit.',
    'T1557'
FROM observation_types WHERE key = 'weak_tls_config';
