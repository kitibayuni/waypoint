CREATE TYPE credential_secret_type AS ENUM ('plaintext', 'ntlm', 'kerb', 'ssh_key', 'hash_other');
CREATE TYPE credential_origin AS ENUM ('captured', 'cracked', 'sprayed', 'default', 'created');

CREATE TABLE credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    username TEXT NOT NULL,
    domain TEXT,
    -- App-level AEAD ciphertext; never stored or returned in plaintext.
    secret BYTEA NOT NULL,
    secret_type credential_secret_type NOT NULL,
    source_host_id UUID REFERENCES hosts(id) ON DELETE SET NULL,
    origin credential_origin NOT NULL DEFAULT 'captured',
    validated BOOLEAN NOT NULL DEFAULT FALSE,
    notes_md TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TYPE credential_usage_result AS ENUM ('works', 'fails', 'untested');
CREATE TYPE credential_privilege AS ENUM ('user', 'admin', 'domain_admin', 'system');

CREATE TABLE credential_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    credential_id UUID NOT NULL REFERENCES credentials(id) ON DELETE CASCADE,
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    service_id UUID REFERENCES services(id) ON DELETE SET NULL,
    result credential_usage_result NOT NULL DEFAULT 'untested',
    privilege credential_privilege,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_credentials_engagement_id ON credentials(engagement_id);
CREATE INDEX idx_credential_usage_credential_id ON credential_usage(credential_id);
CREATE INDEX idx_credential_usage_host_id ON credential_usage(host_id);
