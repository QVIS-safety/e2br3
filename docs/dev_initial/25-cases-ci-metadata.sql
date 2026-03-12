ALTER TABLE cases
    ADD COLUMN IF NOT EXISTS mfds_report_type TEXT,
    ADD COLUMN IF NOT EXISTS report_year VARCHAR(10),
    ADD COLUMN IF NOT EXISTS source_document_name TEXT,
    ADD COLUMN IF NOT EXISTS source_document_base64 TEXT,
    ADD COLUMN IF NOT EXISTS source_document_media_type TEXT;
