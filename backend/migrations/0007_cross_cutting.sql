CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id UUID NOT NULL,
    before JSONB,
    after JSONB,
    at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_log_subject ON audit_log(subject_type, subject_id);

-- Full-text search: generated tsvector columns + GIN indexes per §4.10.
ALTER TABLE notes ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(title, '') || ' ' || coalesce(body_md, ''))) STORED;
CREATE INDEX idx_notes_search ON notes USING GIN (search_vector);

ALTER TABLE findings ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english',
        coalesce(title, '') || ' ' || coalesce(description_md, '') || ' ' || coalesce(remediation_md, '')
    )) STORED;
CREATE INDEX idx_findings_search ON findings USING GIN (search_vector);

ALTER TABLE observations ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(evidence_md, ''))) STORED;
CREATE INDEX idx_observations_search ON observations USING GIN (search_vector);

ALTER TABLE hosts ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english',
        coalesce(label, '') || ' ' || coalesce(hostname, '') || ' ' || coalesce(general_info_md, '')
    )) STORED;
CREATE INDEX idx_hosts_search ON hosts USING GIN (search_vector);

ALTER TABLE credentials ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(username, '') || ' ' || coalesce(domain, ''))) STORED;
CREATE INDEX idx_credentials_search ON credentials USING GIN (search_vector);

ALTER TABLE attachments ADD COLUMN search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(filename, '') || ' ' || coalesce(caption, ''))) STORED;
CREATE INDEX idx_attachments_search ON attachments USING GIN (search_vector);
