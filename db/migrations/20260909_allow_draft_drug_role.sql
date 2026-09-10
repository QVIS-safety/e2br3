-- Drafts can retain an unselected role; submission validation still requires it.
BEGIN;
ALTER TABLE drug_information
    DROP CONSTRAINT IF EXISTS drug_information_drug_characterization_check;
ALTER TABLE drug_information
    ADD CONSTRAINT drug_information_drug_characterization_check
    CHECK (drug_characterization IN ('', '1', '2', '3', '4'));
COMMIT;
