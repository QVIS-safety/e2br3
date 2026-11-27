-- Historical test fixture only; never loaded by database initialization.
-- Expose HOME Workflow Read as a generated action so route/widget gates use
-- the same policy catalog as the existing home.workflow.read grant.
UPDATE authorization_catalog_state
SET schema_version = schema_version + 1,
    catalog_hash = 'c8cca00a14402c5424ad3c2c94307bb972369260c74d051e0e387cb99794e7a2',
    reconciled_at = now()
WHERE singleton = true
  AND catalog_hash = 'a31959327d4d8ebd1a6c643ea2c4b15b5848a19b678e5594b325b207184a6db7';
