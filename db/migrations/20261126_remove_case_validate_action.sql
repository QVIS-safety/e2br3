-- Remove the obsolete validator state-mutation action; grants remain unchanged.
UPDATE authorization_catalog_state
SET catalog_hash = '959e4b7cdf727387eb73515eb0816ead61f3ed4a1124d4ee79a7dfb4d75455ab'
WHERE singleton
  AND catalog_hash = '933bd65fdf8ddbebc6885c9098256b80e8d636941b5fa014774ace637e4224cf';
