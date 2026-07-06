-- Tracks how a pivot was actually established between hosts -- the tunnel
-- mechanism, listener port, and reached network segment -- none of which
-- trust_relationships or hosts.is_pivot capture today. Grounded in
-- pivot-tunnels-proxies/ and enterprise-networks/6,7,9.txt's multi-hop
-- chains (SSH -D/-L/-R, Chisel, Ligolo, sshuttle, socat, Metasploit
-- autoroute+socks_proxy).
--
-- to_host_id is nullable because a tunnel commonly opens up a whole subnet
-- (e.g. `run autoroute -s 172.16.9.0/23`), not one specific host --
-- remote_target holds the CIDR/description in that case.
CREATE TYPE pivot_tunnel_method AS ENUM (
    'ssh_dynamic', 'ssh_local', 'ssh_remote', 'chisel', 'ligolo', 'sshuttle',
    'socat', 'metasploit_autoroute', 'dns_tunnel', 'icmp_tunnel', 'other'
);

CREATE TABLE pivot_tunnels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engagement_id UUID NOT NULL REFERENCES engagements(id) ON DELETE CASCADE,
    from_host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    to_host_id UUID REFERENCES hosts(id) ON DELETE CASCADE,
    method pivot_tunnel_method NOT NULL,
    local_port INTEGER,
    remote_target TEXT,
    notes_md TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_pivot_tunnels_engagement_id ON pivot_tunnels(engagement_id);
