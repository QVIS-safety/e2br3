-- Historical test fixture only; never loaded by database initialization.
-- Advance only the reviewed action catalog that preceded the built-in
-- administrator role-assignment split. Unknown catalog states remain closed.
UPDATE authorization_catalog_state
SET schema_version = schema_version + 1,
    catalog_hash = '3fab94b12bf16fa4c84bdcfa704f55de1279f2e86baebdc6b8f1d2f16916f270',
    reconciled_at = now()
WHERE singleton
  AND catalog_hash = 'd736b50a197d427894318ce08afb0c795f94531dd9f9801c52d519da94a67f53';
