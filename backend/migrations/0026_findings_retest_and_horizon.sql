-- Two independent additions to findings, bundled in one migration since both
-- are small columns on the same table:
--
-- 1. remediation_horizon: lets a finding's remediation be bucketed into the
--    short/medium/long-term roadmap that documentation&reporting/
--    components-of-a-report.txt calls for in a report's "Summary of
--    Recommendations" section -- distinct from the finding's own
--    remediation_md, which is the detailed fix, not the timeline bucket.
-- 2. retest fields: post-remediation retesting is its own phase per
--    documentation&reporting/types-of-reports.txt ("retest ONLY the
--    original findings"). retested_by is nullable (not every finding gets
--    retested) and set-once in practice via the /retest endpoint, not
--    editable through the normal finding update path.

CREATE TYPE remediation_horizon AS ENUM ('short', 'medium', 'long');

ALTER TABLE findings ADD COLUMN remediation_horizon remediation_horizon;
ALTER TABLE findings ADD COLUMN retested_at TIMESTAMPTZ;
ALTER TABLE findings ADD COLUMN retested_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE findings ADD COLUMN retest_notes_md TEXT NOT NULL DEFAULT '';
