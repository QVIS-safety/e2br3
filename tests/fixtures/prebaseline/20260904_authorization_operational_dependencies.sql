-- Historical test fixture only; never loaded by database initialization.
-- Add operational permission dependencies and scoped import/submission option actions.
UPDATE authorization_catalog_state
SET catalog_hash = '933bd65fdf8ddbebc6885c9098256b80e8d636941b5fa014774ace637e4224cf'
WHERE singleton
  AND catalog_hash = 'bcd18e25cd7bc516e7dd2ad4c18591092742edfaa83bf2d0b5574b91ffa8d1c2';
