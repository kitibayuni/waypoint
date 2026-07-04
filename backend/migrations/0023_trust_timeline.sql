-- ADJUSTMENTS.txt v5: relationship creation (via the new drag-to-connect UI or the
-- existing forms) should show up in Replay "as usual", like every other creation
-- does. timeline_events (0018_remove_observations.sql) never had a branch for
-- trust_relationships at all -- this was a gap, not a deliberate omission.
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
  AND (a.before ->> 'status') IS DISTINCT FROM (a.after ->> 'status')

UNION ALL
SELECT tr.engagement_id, tr.discovered_at, 'trust_relationship_added', 'trust_relationship', tr.id,
       fh.label || ' -> ' || th.label, tr.kind::text
FROM trust_relationships tr
JOIN hosts fh ON fh.id = tr.from_host_id
JOIN hosts th ON th.id = tr.to_host_id;
