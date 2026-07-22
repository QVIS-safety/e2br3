-- Reviewed catalog transition: the grant/action semantics are unchanged.
-- The new hash adds registry-owned Role & Privilege UI bindings to the
-- canonical contract. Unknown predecessor hashes remain untouched so startup
-- still fails closed in AuthorizationCatalogRepository::reconcile.
UPDATE authorization_catalog_state
SET catalog_hash = '7c54720ec57d35509fe7bc2d60a47fdacc7efb8df5513f336cb0fc717293f924',
    reconciled_at = now()
WHERE singleton
  AND catalog_hash = 'a8cbca03d528beb474ba7beb0658a5e22103df9b1a2693c9cce22708530a79e5';
