DROP TABLE IF EXISTS receiver_information;

ALTER TABLE safety_report_identification
    DROP COLUMN IF EXISTS receiver_organization;
