-- Historical test fixture only; never loaded by database initialization.
-- User deactivation is now driven by access_end_at; remove manual delete actions.
UPDATE authorization_catalog_state
SET schema_version = schema_version + 1,
    catalog_hash = '4344f2d4c18a675c9eefcd55b6d942671b541fd47249dcd494acc426c2d5dba2',
    reconciled_at = now()
WHERE singleton = true
  AND catalog_hash = 'c8cca00a14402c5424ad3c2c94307bb972369260c74d051e0e387cb99794e7a2';
