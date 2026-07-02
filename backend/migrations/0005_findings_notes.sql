CREATE TYPE finding_status AS ENUM ('open', 'triaged', 'accepted_risk', 'fixed');

CREATE TABLE findings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    cve TEXT,
    cvss_vector TEXT,
    cvss_score NUMERIC(3, 1),
    severity TEXT,
    description_md TEXT NOT NULL DEFAULT '',
    remediation_md TEXT NOT NULL DEFAULT '',
    poc_md TEXT NOT NULL DEFAULT '',
    references_json JSONB NOT NULL DEFAULT '[]',
    status finding_status NOT NULL DEFAULT 'open',
    source_observation_id UUID REFERENCES observations(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE finding_hosts (
    finding_id UUID NOT NULL REFERENCES findings(id) ON DELETE CASCADE,
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    PRIMARY KEY (finding_id, host_id)
);

CREATE TYPE note_subject_type AS ENUM ('engagement', 'host', 'finding', 'observation', 'credential');

CREATE TABLE notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    subject_type note_subject_type NOT NULL,
    subject_id UUID NOT NULL,
    title TEXT,
    body_md TEXT NOT NULL DEFAULT '',
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE attachment_subject_type AS ENUM ('engagement', 'host', 'finding', 'observation', 'credential', 'note');

CREATE TABLE attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    subject_type attachment_subject_type NOT NULL,
    subject_id UUID NOT NULL,
    filename TEXT NOT NULL,
    mime TEXT,
    size BIGINT,
    storage_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    caption TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE checklists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_id UUID REFERENCES hosts(id) ON DELETE CASCADE,
    engagement_id UUID REFERENCES engagements(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    -- FK to templates added in 0006_templates.sql once that table exists.
    template_origin_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (host_id IS NOT NULL OR engagement_id IS NOT NULL)
);

CREATE TYPE checklist_item_state AS ENUM ('todo', 'doing', 'done', 'na');

CREATE TABLE checklist_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    checklist_id UUID NOT NULL REFERENCES checklists(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    state checklist_item_state NOT NULL DEFAULT 'todo',
    position INTEGER NOT NULL DEFAULT 0,
    linked_note_id UUID REFERENCES notes(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_findings_engagement_id ON findings(engagement_id);
CREATE INDEX idx_notes_engagement_id ON notes(engagement_id);
CREATE INDEX idx_notes_subject ON notes(subject_type, subject_id);
CREATE INDEX idx_attachments_engagement_id ON attachments(engagement_id);
CREATE INDEX idx_attachments_subject ON attachments(subject_type, subject_id);
CREATE INDEX idx_checklists_host_id ON checklists(host_id);
CREATE INDEX idx_checklists_engagement_id ON checklists(engagement_id);
CREATE INDEX idx_checklist_items_checklist_id ON checklist_items(checklist_id);
