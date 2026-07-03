-- Full removal of the observations/observation_types/attack_path_rules feature and the
-- attack-path suggestion engine built on top of it, per explicit request ("get rid of
-- the observations ... and any code that has the observations functionality"). The
-- host-page UI for this was already removed; this migration retires the remaining
-- backend schema/data. This is destructive for local/dev data -- there is no production
-- engagement data against this schema yet.

-- Drop any notes/attachments attached to an observation (would otherwise be orphaned
-- once observations is dropped) before narrowing the subject_type enums below.
DELETE FROM notes WHERE subject_type = 'observation';
DELETE FROM attachments WHERE subject_type = 'observation';

ALTER TYPE note_subject_type RENAME TO note_subject_type_old;
CREATE TYPE note_subject_type AS ENUM ('engagement', 'host', 'finding', 'credential');
ALTER TABLE notes ALTER COLUMN subject_type TYPE note_subject_type USING subject_type::text::note_subject_type;
DROP TYPE note_subject_type_old;

ALTER TYPE attachment_subject_type RENAME TO attachment_subject_type_old;
CREATE TYPE attachment_subject_type AS ENUM ('engagement', 'host', 'finding', 'credential', 'note');
ALTER TABLE attachments ALTER COLUMN subject_type TYPE attachment_subject_type USING subject_type::text::attachment_subject_type;
DROP TYPE attachment_subject_type_old;

-- Rebuild timeline_events without the observation-derived branches.
DROP VIEW timeline_events;
CREATE VIEW timeline_events AS
SELECT h.engagement_id, h.created_at AS at, 'host_added' AS event_type,
       'host' AS subject_type, h.id AS subject_id, h.label AS title, NULL::text AS summary
FROM hosts h

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
SELECT f.engagement_id, a.at, 'finding_' || (a.after ->> 'status'), 'finding', a.subject_id,
       f.title,
       'status: ' || COALESCE(a.before ->> 'status', '?') || ' -> ' || COALESCE(a.after ->> 'status', '?')
FROM audit_log a
JOIN findings f ON f.id = a.subject_id
WHERE a.subject_type = 'finding' AND a.action = 'update'
  AND (a.before ->> 'status') IS DISTINCT FROM (a.after ->> 'status');

-- Drop the observation-dependent findings column, then the tables/type themselves, in FK order.
ALTER TABLE findings DROP COLUMN source_observation_id;
DROP TABLE attack_path_rules;
DROP TABLE observations;
DROP TABLE observation_types;
DROP TYPE observation_status;
