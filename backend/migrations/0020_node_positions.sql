CREATE TABLE node_positions (
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    x DOUBLE PRECISION NOT NULL,
    y DOUBLE PRECISION NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (engagement_id, node_id)
);
