DROP TRIGGER IF EXISTS guard_receiver_information_source_receiver_presave
    ON receiver_information;
DROP INDEX IF EXISTS idx_receiver_info_source_presave;
ALTER TABLE receiver_information
    DROP COLUMN IF EXISTS source_receiver_presave_id;
