-- ADJUSTMENTS.txt v11: "When services are added, please log this (the replay
-- feed should show this), and the same when a credential is created/
-- associated with a specific service or host node." Services never had a
-- timeline_events branch at all (a gap, like trust_relationships before
-- 0023_trust_timeline.sql); credential_captured already existed but its
-- summary was always NULL, so a credential's host/service origin was
-- invisible in the feed.
DROP VIEW timeline_events;
CREATE VIEW timeline_events AS
SELECT h.engagement_id, h.created_at AS at, 'host_added' AS event_type,
       'host' AS subject_type, h.id AS subject_id, h.label AS title, NULL::text AS summary
FROM hosts h

UNION ALL
SELECT s.engagement_id, s.at, 'credential_captured', 'credential', s.id, s.title, s.summary
FROM (
    SELECT h2.engagement_id, c.created_at AS at, c.id,
           c.username || COALESCE('@' || c.domain, '') AS title,
           CASE
               WHEN c.source_service_id IS NOT NULL THEN
                   'via ' || COALESCE(sv.display_name, sv.name, 'service') || ' on ' || h2.label
               WHEN c.source_host_id IS NOT NULL THEN 'via ' || h2.label
               ELSE NULL
           END AS summary
    FROM credentials c
    LEFT JOIN hosts h2 ON h2.id = c.source_host_id
    LEFT JOIN services sv ON sv.id = c.source_service_id
) s

UNION ALL
SELECT h.engagement_id, s.created_at, 'service_added', 'service', s.id,
       h.label || ': ' || COALESCE(s.display_name, s.name, 'service'),
       s.port || '/' || s.protocol::text
FROM services s
JOIN hosts h ON h.id = s.host_id

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
