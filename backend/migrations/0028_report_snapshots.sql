-- Freezes a report's rendered HTML at a point in time, distinct from the
-- existing GET /reports/:id (which always renders current live data -- a
-- "preview", not a delivered artifact). types-of-reports.txt: some auditors
-- only accept a "Final" report, and a draft/final distinction lets a
-- post-remediation report later diff against what was actually delivered
-- rather than whatever the live data happens to be by then.
--
-- status is a one-way transition (draft -> final); enforced at the route
-- layer (reject re-marking an already-final snapshot), not with a DB
-- constraint, since Postgres CHECK constraints can't reference the
-- previous row value.
CREATE TYPE report_snapshot_status AS ENUM ('draft', 'final');

CREATE TABLE report_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status report_snapshot_status NOT NULL DEFAULT 'draft',
    html_body TEXT NOT NULL,
    generated_by UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_report_snapshots_engagement_id ON report_snapshots(engagement_id);
