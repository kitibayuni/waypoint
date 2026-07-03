-- ADJUSTMENTS.txt #2: a dedicated free-text section for how a host was accessed,
-- separate from the general-purpose `general_info_md`. The "accessible from" log
-- itself is derived from `trust_relationships` (not stored here) so it can never drift
-- out of sync with the attack graph.
ALTER TABLE hosts ADD COLUMN login_notes_md TEXT NOT NULL DEFAULT '';
