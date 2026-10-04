-- Unknown dispatch has no fabricated remote identifier or acknowledgement.
ALTER TABLE case_submissions ALTER COLUMN remote_submission_id DROP NOT NULL;
ALTER TABLE case_submissions DROP CONSTRAINT case_submission_status_valid;
ALTER TABLE case_submissions ADD CONSTRAINT case_submission_status_valid CHECK (
    status IN ('dispatch_unknown', 'submitted_ack1_pending', 'ack1_received', 'ack2_received', 'ack3_received', 'ack4_received', 'rejected')
);
