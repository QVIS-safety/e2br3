-- Historical test fixture only; never loaded by database initialization.
-- Keep historical diagnostics unchanged; never infer a public classification from text.
ALTER TABLE xml_import_history ADD COLUMN public_error JSONB;
ALTER TABLE xml_import_history ADD CONSTRAINT xml_import_history_public_error_object
    CHECK (public_error IS NULL OR (status = 'error' AND jsonb_typeof(public_error) = 'object'));
