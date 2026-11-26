-- Remove the obsolete submission receiver option-list action.
UPDATE authorization_catalog_state
SET catalog_hash = '3a652cf826232fd5f93f7a1c27cb08aa4afd372f0236868414dc26f3b81a05be'
WHERE singleton
  AND catalog_hash = '959e4b7cdf727387eb73515eb0816ead61f3ed4a1124d4ee79a7dfb4d75455ab';
