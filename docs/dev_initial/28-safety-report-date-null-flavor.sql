ALTER TABLE safety_report_identification
ADD COLUMN IF NOT EXISTS transmission_date_null_flavor VARCHAR(4)
    CHECK (transmission_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS date_first_received_from_source_null_flavor VARCHAR(4)
    CHECK (date_first_received_from_source_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
ADD COLUMN IF NOT EXISTS date_of_most_recent_information_null_flavor VARCHAR(4)
    CHECK (date_of_most_recent_information_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK'));

ALTER TABLE safety_report_identification
ALTER COLUMN transmission_date DROP NOT NULL,
ALTER COLUMN date_first_received_from_source DROP NOT NULL,
ALTER COLUMN date_of_most_recent_information DROP NOT NULL;
