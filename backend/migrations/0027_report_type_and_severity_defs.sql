-- report_type drives which sections render/the sections' content shape in
-- report::render_html (documentation&reporting/types-of-reports.txt:
-- Vulnerability Assessment, full Penetration Test, Attestation, and
-- Post-Remediation reports have materially different structures, not just
-- different content within the same structure). Defaults to the fullest
-- report shape (penetration_test) so existing engagements/reports don't
-- change behavior until someone deliberately picks a narrower type.
--
-- severity_definitions_md is a per-engagement editable block (seeded with a
-- sensible default) rendered as its own report appendix -- components-of-a-
-- report.txt lists "Severity Ratings: define your rating criteria" as a
-- static appendix every report should have.
CREATE TYPE report_type AS ENUM ('vuln_assessment', 'penetration_test', 'attestation', 'post_remediation');

ALTER TABLE engagements ADD COLUMN report_type report_type NOT NULL DEFAULT 'penetration_test';
ALTER TABLE engagements ADD COLUMN severity_definitions_md TEXT NOT NULL DEFAULT '**Critical** — immediate, unauthenticated compromise of confidentiality, integrity, or availability at scale.
**High** — significant compromise requiring minimal precondition or user interaction.
**Medium** — meaningful risk requiring specific conditions or limited impact.
**Low** — minor risk, defense-in-depth, or best-practice deviation.';
