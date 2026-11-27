-- Historical test fixture only; never loaded by database initialization.
-- Export execution and export-history reads now have separate contextual
-- actions under the existing Export/Submission Edit and View grants. Import
-- history detail reads also enforce the request principal's Case scope. Notice
-- View is now an explicit action instead of a legacy field-level permission.
UPDATE authorization_catalog_state
SET schema_version = 3,
    catalog_hash = 'd736b50a197d427894318ce08afb0c795f94531dd9f9801c52d519da94a67f53',
    reconciled_at = now()
WHERE singleton
  AND catalog_hash = 'c26d5ff8651378fb41b725593cbe755f7bd0e290ce06e5c6dc030b74e96f4c58';
