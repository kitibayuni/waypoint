CREATE TYPE template_kind AS ENUM ('host', 'checklist', 'finding', 'note', 'engagement');

CREATE TABLE templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind template_kind NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    owner_id UUID REFERENCES users(id) ON DELETE SET NULL,
    is_shared BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE template_payloads (
    template_id UUID PRIMARY KEY REFERENCES templates(id) ON DELETE CASCADE,
    body JSONB NOT NULL
);

-- Deferred from 0002_assets.sql and 0005_findings_notes.sql, now that
-- templates exists.
ALTER TABLE hosts
    ADD CONSTRAINT fk_hosts_template_origin
    FOREIGN KEY (template_origin_id) REFERENCES templates(id) ON DELETE SET NULL;

ALTER TABLE checklists
    ADD CONSTRAINT fk_checklists_template_origin
    FOREIGN KEY (template_origin_id) REFERENCES templates(id) ON DELETE SET NULL;

CREATE INDEX idx_templates_kind ON templates(kind);
