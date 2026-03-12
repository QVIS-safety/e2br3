UPDATE primary_sources
SET primary_source_regulatory = '2'
WHERE primary_source_regulatory = '3';

ALTER TABLE primary_sources
DROP CONSTRAINT IF EXISTS primary_sources_primary_source_regulatory_check;

ALTER TABLE primary_sources
ADD CONSTRAINT primary_sources_primary_source_regulatory_check
CHECK (
    primary_source_regulatory IS NULL
    OR primary_source_regulatory IN ('1', '2')
);

DELETE FROM terminology
WHERE category = 'primary_source_regulatory'
  AND code = '3';
