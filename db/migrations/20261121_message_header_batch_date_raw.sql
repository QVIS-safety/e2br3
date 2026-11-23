ALTER TABLE message_headers
    ADD COLUMN batch_transmission_date_raw TEXT;

ALTER TABLE message_headers
    ADD CONSTRAINT message_headers_batch_date_single_representation
    CHECK (batch_transmission_date_raw IS NULL OR
           (batch_transmission_date IS NULL AND batch_transmission_date_raw <> ''));

-- Recover previously discarded N.1.5 from the retained import, never from now().
DO $$
DECLARE
    imported RECORD;
    dates XML[];
BEGIN
    PERFORM set_config('app.current_user_id', '00000000-0000-0000-0000-000000000001', true);
    PERFORM set_config('app.current_organization_id', '00000000-0000-0000-0000-000000000000', true);
    PERFORM set_config('app.current_user_role', 'system_admin', true);
    PERFORM set_config('app.platform_isolation_bypass', 'true', true);
    PERFORM set_config('app.change_reason', 'Recover imported N.1.5 from retained original XML', true);

    FOR imported IN
        SELECT h.id, h.case_id, c.raw_xml
          FROM message_headers h JOIN cases c ON c.id = h.case_id
         WHERE h.batch_transmission_date IS NULL
           AND h.batch_transmission_date_raw IS NULL
           AND c.raw_xml IS NOT NULL
    LOOP
        dates := xpath(
            '/h:MCCI_IN200100UV01/h:creationTime/@value',
            XMLPARSE(DOCUMENT convert_from(imported.raw_xml, 'UTF8')),
            ARRAY[ARRAY['h', 'urn:hl7-org:v3']]
        );
        IF cardinality(dates) > 1 THEN
            RAISE EXCEPTION 'Ambiguous original N.1.5 for case %', imported.case_id;
        ELSIF cardinality(dates) = 1 THEN
            IF btrim(dates[1]::text) = '' THEN
                RAISE EXCEPTION 'Empty original N.1.5 for case %', imported.case_id;
            END IF;
            UPDATE message_headers
               SET batch_transmission_date_raw = dates[1]::text
             WHERE id = imported.id;
        END IF;
    END LOOP;
END
$$;
