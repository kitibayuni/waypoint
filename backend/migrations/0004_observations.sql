CREATE TABLE observation_types (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    category TEXT NOT NULL,
    default_severity TEXT NOT NULL,
    description_md TEXT NOT NULL DEFAULT '',
    references_json JSONB NOT NULL DEFAULT '[]',
    mitre_technique_ids JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE observation_status AS ENUM ('suspected', 'confirmed', 'remediated', 'false_positive');

CREATE TABLE observations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    service_id UUID REFERENCES services(id) ON DELETE SET NULL,
    observation_type_id UUID NOT NULL REFERENCES observation_types(id) ON DELETE RESTRICT,
    severity_override TEXT,
    status observation_status NOT NULL DEFAULT 'suspected',
    evidence_md TEXT NOT NULL DEFAULT '',
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE attack_path_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trigger_observation_type_id UUID NOT NULL REFERENCES observation_types(id) ON DELETE CASCADE,
    technique TEXT NOT NULL,
    outcome TEXT NOT NULL,
    next_step_md TEXT NOT NULL DEFAULT '',
    mitre_technique_id TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE trust_relationship_kind AS ENUM ('domain_trust', 'admin_of', 'shares_creds', 'session');

CREATE TABLE trust_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    from_host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    to_host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    kind trust_relationship_kind NOT NULL,
    direction TEXT,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_observations_host_id ON observations(host_id);
CREATE INDEX idx_observations_observation_type_id ON observations(observation_type_id);
CREATE INDEX idx_attack_path_rules_trigger ON attack_path_rules(trigger_observation_type_id);
CREATE INDEX idx_trust_relationships_engagement_id ON trust_relationships(engagement_id);
