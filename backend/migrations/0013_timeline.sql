-- Phase 16: Timeline & Replay view (DESIGN.md §8).
-- Replay filters graph queries by these "when did this become true" columns
-- rather than created_at, since a row's mere existence isn't the same as
-- the state it represents becoming true (e.g. an observation can exist as
-- 'suspected' long before it's confirmed).
ALTER TABLE observations ADD COLUMN confirmed_at TIMESTAMPTZ;
UPDATE observations SET confirmed_at = created_at WHERE status = 'confirmed';

ALTER TABLE credential_usage ADD COLUMN tested_at TIMESTAMPTZ;
UPDATE credential_usage SET tested_at = created_at WHERE result <> 'untested';

-- Trust relationships have no separate "discovery" step distinct from
-- creation in this app today, so discovered_at simply defaults to now()
-- (== created_at) for every row, existing and future.
ALTER TABLE trust_relationships ADD COLUMN discovered_at TIMESTAMPTZ NOT NULL DEFAULT now();
UPDATE trust_relationships SET discovered_at = created_at;

-- Unified, derived (not stored) timeline feed: creation events from every
-- timestamped entity, plus status-change events sourced from audit_log.
-- event_type is generic ("<subject_type>_<new status>") so it reproduces
-- examples like observation_confirmed / finding_triaged without a fixed enum.
CREATE VIEW timeline_events AS
SELECT h.engagement_id, h.created_at AS at, 'host_added' AS event_type,
       'host' AS subject_type, h.id AS subject_id, h.label AS title, NULL::text AS summary
FROM hosts h

UNION ALL
SELECT h.engagement_id, o.created_at, 'observation_added', 'observation', o.id, ot.title, NULL
FROM observations o
JOIN observation_types ot ON ot.id = o.observation_type_id
JOIN hosts h ON h.id = o.host_id

UNION ALL
SELECT c.engagement_id, c.created_at, 'credential_captured', 'credential', c.id,
       c.username || COALESCE('@' || c.domain, ''), NULL
FROM credentials c

UNION ALL
SELECT n.engagement_id, n.created_at, 'note_added', 'note', n.id, COALESCE(n.title, 'Note'), NULL
FROM notes n

UNION ALL
SELECT f.engagement_id, f.created_at, 'finding_added', 'finding', f.id, f.title, NULL
FROM findings f

UNION ALL
SELECT h.engagement_id, a.at, 'observation_' || (a.after ->> 'status'), 'observation', a.subject_id,
       ot.title,
       'status: ' || COALESCE(a.before ->> 'status', '?') || ' -> ' || COALESCE(a.after ->> 'status', '?')
FROM audit_log a
JOIN observations o ON o.id = a.subject_id
JOIN hosts h ON h.id = o.host_id
JOIN observation_types ot ON ot.id = o.observation_type_id
WHERE a.subject_type = 'observation' AND a.action = 'update'
  AND (a.before ->> 'status') IS DISTINCT FROM (a.after ->> 'status')

UNION ALL
SELECT f.engagement_id, a.at, 'finding_' || (a.after ->> 'status'), 'finding', a.subject_id,
       f.title,
       'status: ' || COALESCE(a.before ->> 'status', '?') || ' -> ' || COALESCE(a.after ->> 'status', '?')
FROM audit_log a
JOIN findings f ON f.id = a.subject_id
WHERE a.subject_type = 'finding' AND a.action = 'update'
  AND (a.before ->> 'status') IS DISTINCT FROM (a.after ->> 'status');
