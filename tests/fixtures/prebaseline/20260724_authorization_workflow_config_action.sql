-- Historical test fixture only; never loaded by database initialization.
-- CASE Workflow Read now also protects the workflow runtime configuration
-- endpoint through the canonical case.workflow.config.read action.
UPDATE authorization_catalog_state
SET schema_version = 3,
    catalog_hash = 'c26d5ff8651378fb41b725593cbe755f7bd0e290ce06e5c6dc030b74e96f4c58',
    reconciled_at = now()
WHERE singleton
  AND catalog_hash = '6d3135091fbb99216747a104abb5d90e460d732b5e73c5845a621f075d602504';
