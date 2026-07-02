CREATE TYPE host_status AS ENUM ('discovered', 'enumerating', 'exploited', 'owned', 'cleared');

CREATE TABLE hosts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    hostname TEXT,
    os TEXT,
    os_family TEXT,
    criticality TEXT,
    status host_status NOT NULL DEFAULT 'discovered',
    general_info_md TEXT NOT NULL DEFAULT '',
    -- FK to templates added in 0006_templates.sql once that table exists.
    template_origin_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE host_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    ip INET NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TYPE service_protocol AS ENUM ('tcp', 'udp');

CREATE TABLE services (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    port INTEGER NOT NULL CHECK (port BETWEEN 0 AND 65535),
    protocol service_protocol NOT NULL DEFAULT 'tcp',
    name TEXT,
    product TEXT,
    version TEXT,
    banner TEXT,
    state TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    UNIQUE (engagement_id, name)
);

CREATE TABLE host_tags (
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (host_id, tag_id)
);

CREATE INDEX idx_hosts_engagement_id ON hosts(engagement_id);
CREATE INDEX idx_host_addresses_host_id ON host_addresses(host_id);
CREATE INDEX idx_services_host_id ON services(host_id);
CREATE INDEX idx_tags_engagement_id ON tags(engagement_id);
