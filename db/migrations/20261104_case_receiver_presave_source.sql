ALTER TABLE receiver_information
    ADD COLUMN IF NOT EXISTS source_receiver_presave_id UUID
        REFERENCES receiver_presaves(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_receiver_info_source_presave
    ON receiver_information(source_receiver_presave_id);

DROP TRIGGER IF EXISTS guard_receiver_information_source_receiver_presave
    ON receiver_information;
CREATE TRIGGER guard_receiver_information_source_receiver_presave
BEFORE INSERT OR UPDATE OF source_receiver_presave_id ON receiver_information
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'receiver_presaves', 'source_receiver_presave_id', 'case'
);
