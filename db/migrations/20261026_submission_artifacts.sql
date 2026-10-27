-- Legacy attempts remain unknown: current Case XML is not proof of transmitted bytes.
ALTER TABLE case_submissions
    ADD COLUMN authority TEXT CHECK (authority IN ('fda', 'mfds')),
    ADD COLUMN submitted_xml BYTEA,
    ADD CONSTRAINT submission_authority_transport CHECK (
        authority IS NULL
        OR (authority = 'fda' AND gateway IN ('as2-submitter-http-fda', 'fda-esg-nextgen-api'))
        OR (authority = 'mfds' AND gateway = 'as2-submitter-http-mfds')
    ),
    ADD CONSTRAINT submission_artifact_complete CHECK (
        (authority IS NULL AND submitted_xml IS NULL)
        OR (authority IS NOT NULL AND submitted_xml IS NOT NULL
            AND octet_length(submitted_xml) > 0 AND octet_length(submitted_xml) = xml_bytes)
    );

CREATE FUNCTION protect_submission_artifact() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.authority, NEW.submitted_xml, NEW.case_id, NEW.gateway, NEW.xml_bytes)
        IS DISTINCT FROM
       (OLD.authority, OLD.submitted_xml, OLD.case_id, OLD.gateway, OLD.xml_bytes) THEN
        RAISE EXCEPTION 'Submission identity and original XML are immutable';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER protect_submission_artifact
    BEFORE UPDATE ON case_submissions
    FOR EACH ROW EXECUTE FUNCTION protect_submission_artifact();
