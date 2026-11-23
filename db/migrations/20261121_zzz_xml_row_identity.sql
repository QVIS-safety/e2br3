-- External XML identities are case-local, not cross-organization primary keys.
-- Older imports have no recoverable mapping here; never invent one from local IDs.
ALTER TABLE reactions ADD COLUMN source_xml_id TEXT
    CHECK (source_xml_id IS NULL OR length(btrim(source_xml_id)) > 0);
ALTER TABLE drug_information ADD COLUMN source_xml_id TEXT
    CHECK (source_xml_id IS NULL OR length(btrim(source_xml_id)) > 0);
CREATE UNIQUE INDEX reactions_case_source_xml_id
    ON reactions(case_id, source_xml_id) WHERE source_xml_id IS NOT NULL;
CREATE UNIQUE INDEX drug_information_case_source_xml_id
    ON drug_information(case_id, source_xml_id) WHERE source_xml_id IS NOT NULL;
