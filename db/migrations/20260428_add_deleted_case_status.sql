ALTER TABLE cases DROP CONSTRAINT IF EXISTS case_status_valid;

ALTER TABLE cases
    ADD CONSTRAINT case_status_valid
    CHECK (status IN ('draft', 'reviewed', 'validated', 'locked', 'submitted', 'deleted', 'archived', 'nullified'));
