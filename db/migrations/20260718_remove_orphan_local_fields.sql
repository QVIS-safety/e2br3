DROP TABLE IF EXISTS drug_recurrence_information;

ALTER TABLE patient_information
    DROP COLUMN IF EXISTS patient_given_name,
    DROP COLUMN IF EXISTS patient_family_name;

ALTER TABLE drug_information
    DROP COLUMN IF EXISTS brand_name,
    DROP COLUMN IF EXISTS drug_generic_name,
    DROP COLUMN IF EXISTS rechallenge,
    DROP COLUMN IF EXISTS parent_dosage_text;

ALTER TABLE dosage_information
    DROP COLUMN IF EXISTS first_administration_time,
    DROP COLUMN IF EXISTS last_administration_time;

ALTER TABLE drug_reaction_assessments
    DROP COLUMN IF EXISTS recurrence_meddra_version,
    DROP COLUMN IF EXISTS recurrence_meddra_code;
