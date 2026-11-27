-- Historical test fixture only; never loaded by database initialization.
-- Align the persisted authorization catalog with the audit grant registry.
UPDATE authorization_catalog_state
SET catalog_hash = 'bcd18e25cd7bc516e7dd2ad4c18591092742edfaa83bf2d0b5574b91ffa8d1c2'
WHERE singleton
  AND catalog_hash = 'fc350e96458cd18122ec5ee07f0e8913d523924c6f9b6e2b78fade758376070c';
