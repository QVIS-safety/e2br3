-- Historical test fixture only; never loaded by database initialization.
-- Allow active users to read reference terminology without admin privileges.
-- The action catalog hash must advance with the registry contract.
UPDATE authorization_catalog_state
SET catalog_hash = 'fc350e96458cd18122ec5ee07f0e8913d523924c6f9b6e2b78fade758376070c'
WHERE singleton
  AND catalog_hash = '4344f2d4c18a675c9eefcd55b6d942671b541fd47249dcd494acc426c2d5dba2';
