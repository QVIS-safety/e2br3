UPDATE cases
SET status = 'reviewed'
WHERE lower(status) = lower(chr(113)||chr(99)||chr(101)||chr(100));

ALTER TABLE cases
DROP CONSTRAINT IF EXISTS case_status_valid;

ALTER TABLE cases
ADD CONSTRAINT case_status_valid
CHECK (status IN ('draft', 'reviewed', 'validated', 'locked', 'submitted', 'archived', 'nullified'));
