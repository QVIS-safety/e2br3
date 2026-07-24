-- Reviewed catalog transition: actions now refer directly to the PDF grant
-- identifiers. The removed entitlement layer did not add independent policy
-- choices, so persisted role_grants remain unchanged.
UPDATE authorization_catalog_state
SET catalog_hash = '0f0ee103d4ebf9f448c16d68cf5a7e11cfa8c08b4f723845dfc7db44764c66eb',
    reconciled_at = now()
WHERE singleton
  AND catalog_hash = '7c54720ec57d35509fe7bc2d60a47fdacc7efb8df5513f336cb0fc717293f924';
