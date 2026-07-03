-- ADJUSTMENTS.txt v2: replace the Observations host-page workflow with a lighter
-- per-checklist-item risk flag. 'safe' auto-completes the item (crossed out text);
-- 'exploit' also auto-completes it but is rendered reddish/bold to flag a confirmed
-- finding-worthy result right on the checklist.
CREATE TYPE checklist_item_assessment AS ENUM ('safe', 'undecided', 'exploit');
ALTER TABLE checklist_items ADD COLUMN assessment checklist_item_assessment NOT NULL DEFAULT 'undecided';
