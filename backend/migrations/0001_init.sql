CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    contacts JSONB NOT NULL DEFAULT '[]',
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE engagement_status AS ENUM ('planning', 'active', 'reporting', 'closed');

CREATE TABLE engagements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id UUID NOT NULL REFERENCES clients(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    status engagement_status NOT NULL DEFAULT 'planning',
    start_date DATE,
    end_date DATE,
    global_notes_md TEXT NOT NULL DEFAULT '',
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE engagement_role AS ENUM ('lead', 'tester', 'viewer');

CREATE TABLE engagement_members (
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role engagement_role NOT NULL DEFAULT 'tester',
    added_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (engagement_id, user_id)
);

CREATE TYPE scope_item_kind AS ENUM ('ip', 'cidr', 'domain', 'url', 'asn', 'exclusion');

CREATE TABLE scope_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    kind scope_item_kind NOT NULL,
    value TEXT NOT NULL,
    in_scope BOOLEAN NOT NULL DEFAULT TRUE,
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_engagements_client_id ON engagements(client_id);
CREATE INDEX idx_engagement_members_user_id ON engagement_members(user_id);
CREATE INDEX idx_scope_items_engagement_id ON scope_items(engagement_id);
