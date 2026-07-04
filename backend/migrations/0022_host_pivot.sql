-- ADJUSTMENTS.txt v5: mark a host as a pivot point (colored yellow-orange on the
-- attack graph), distinct from is_foothold's red initial-access marker -- a host can
-- be both, in which case foothold wins visually (see AttackGraph.svelte buildStyle()).
ALTER TABLE hosts ADD COLUMN is_pivot BOOLEAN NOT NULL DEFAULT FALSE;
