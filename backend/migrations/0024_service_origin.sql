-- ADJUSTMENTS.txt v6: service nodes on the attack graph, with the ability to
-- attribute a newly-discovered host/credential to the specific service it came
-- from (rather than just the owning host, which credentials already track via
-- source_host_id). Set only at creation time from the graph's service context menu.
ALTER TABLE hosts ADD COLUMN source_service_id UUID REFERENCES services(id) ON DELETE SET NULL;
ALTER TABLE credentials ADD COLUMN source_service_id UUID REFERENCES services(id) ON DELETE SET NULL;
