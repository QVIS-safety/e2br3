CREATE OR REPLACE FUNCTION require_active_presave_reference()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    referenced_id UUID;
    owning_organization_id UUID;
    active_id UUID;
BEGIN
    referenced_id := NULLIF(to_jsonb(NEW) ->> TG_ARGV[1], '')::UUID;
    IF referenced_id IS NULL THEN
        RETURN NEW;
    END IF;

    CASE TG_ARGV[2]
        WHEN 'organization' THEN
            owning_organization_id :=
                NULLIF(to_jsonb(NEW) ->> 'organization_id', '')::UUID;
        WHEN 'case' THEN
            SELECT organization_id
              INTO owning_organization_id
              FROM cases
             WHERE id = NULLIF(to_jsonb(NEW) ->> 'case_id', '')::UUID;
        WHEN 'study' THEN
            SELECT organization_id
              INTO owning_organization_id
              FROM study_presaves
             WHERE id = NULLIF(to_jsonb(NEW) ->> 'study_presave_id', '')::UUID;
        ELSE
            RAISE EXCEPTION 'unsupported presave owner source: %', TG_ARGV[2];
    END CASE;

    EXECUTE format(
        'SELECT id FROM %I WHERE id = $1 AND organization_id = $2 AND deleted = false FOR KEY SHARE',
        TG_ARGV[0]
    )
    INTO active_id
    USING referenced_id, owning_organization_id;

    IF active_id IS NULL THEN
        RAISE EXCEPTION USING
            ERRCODE = 'P2001',
            MESSAGE = 'inactive presave reference',
            DETAIL = format('%s:%s', TG_ARGV[0], referenced_id);
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS guard_sender_information_source_sender_presave ON sender_information;
CREATE TRIGGER guard_sender_information_source_sender_presave
BEFORE INSERT OR UPDATE OF source_sender_presave_id ON sender_information
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'sender_presaves', 'source_sender_presave_id', 'case'
);

DROP TRIGGER IF EXISTS guard_drug_information_source_product_presave ON drug_information;
CREATE TRIGGER guard_drug_information_source_product_presave
BEFORE INSERT OR UPDATE OF source_product_presave_id ON drug_information
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'product_presaves', 'source_product_presave_id', 'case'
);

DROP TRIGGER IF EXISTS guard_study_information_source_study_presave ON study_information;
CREATE TRIGGER guard_study_information_source_study_presave
BEFORE INSERT OR UPDATE OF source_study_presave_id ON study_information
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'study_presaves', 'source_study_presave_id', 'case'
);

DROP TRIGGER IF EXISTS guard_primary_sources_source_reporter_presave ON primary_sources;
CREATE TRIGGER guard_primary_sources_source_reporter_presave
BEFORE INSERT OR UPDATE OF source_reporter_presave_id ON primary_sources
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'reporter_presaves', 'source_reporter_presave_id', 'case'
);

DROP TRIGGER IF EXISTS guard_narrative_information_source_narrative_presave ON narrative_information;
CREATE TRIGGER guard_narrative_information_source_narrative_presave
BEFORE INSERT OR UPDATE OF source_narrative_presave_id ON narrative_information
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'narrative_presaves', 'source_narrative_presave_id', 'case'
);

DROP TRIGGER IF EXISTS guard_product_presaves_sender_presave ON product_presaves;
CREATE TRIGGER guard_product_presaves_sender_presave
BEFORE INSERT OR UPDATE OF sender_presave_id ON product_presaves
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'sender_presaves', 'sender_presave_id', 'organization'
);

DROP TRIGGER IF EXISTS guard_product_presaves_receiver_presave ON product_presaves;
CREATE TRIGGER guard_product_presaves_receiver_presave
BEFORE INSERT OR UPDATE OF receiver_presave_id ON product_presaves
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'receiver_presaves', 'receiver_presave_id', 'organization'
);

DROP TRIGGER IF EXISTS guard_study_presaves_product_presave ON study_presaves;
CREATE TRIGGER guard_study_presaves_product_presave
BEFORE INSERT OR UPDATE OF product_presave_id ON study_presaves
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'product_presaves', 'product_presave_id', 'organization'
);

DROP TRIGGER IF EXISTS guard_study_presave_products_product_presave ON study_presave_products;
CREATE TRIGGER guard_study_presave_products_product_presave
BEFORE INSERT OR UPDATE OF product_presave_id ON study_presave_products
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'product_presaves', 'product_presave_id', 'study'
);

DROP TRIGGER IF EXISTS guard_study_presave_reporters_reporter_presave ON study_presave_reporters;
CREATE TRIGGER guard_study_presave_reporters_reporter_presave
BEFORE INSERT OR UPDATE OF reporter_presave_id ON study_presave_reporters
FOR EACH ROW EXECUTE FUNCTION require_active_presave_reference(
    'reporter_presaves', 'reporter_presave_id', 'study'
);

