-- Historical test fixture only; never loaded by database initialization.
-- HOME Workflow displays the case work list, so its reviewed Read grant must
-- expand to the canonical CASE read grant used by the list endpoint.
UPDATE authorization_catalog_state
SET schema_version = schema_version + 1,
    catalog_hash = 'a31959327d4d8ebd1a6c643ea2c4b15b5848a19b678e5594b325b207184a6db7',
    reconciled_at = now()
WHERE singleton = true
  AND catalog_hash = '3fab94b12bf16fa4c84bdcfa704f55de1279f2e86baebdc6b8f1d2f16916f270';
