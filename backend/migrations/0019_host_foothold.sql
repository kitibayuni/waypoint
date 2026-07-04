-- ADJUSTMENTS.txt v3: mark a host as the foothold/initial-access point of the
-- engagement, so the attack graph can highlight it distinctly (in red) from
-- hosts reached by pivoting.
ALTER TABLE hosts ADD COLUMN is_foothold BOOLEAN NOT NULL DEFAULT FALSE;
