-- A draft can cross organizations before its sender assigns a report number.
ALTER TABLE safety_report_identification ALTER COLUMN safety_report_id DROP NOT NULL;
-- Preserve the sender's timestamp precision; the input contract validates its syntax.
ALTER TABLE safety_report_identification
    DROP CONSTRAINT safety_report_identification_transmission_date_check,
    ALTER COLUMN transmission_date TYPE TEXT;
