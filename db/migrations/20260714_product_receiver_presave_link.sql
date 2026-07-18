ALTER TABLE product_presaves
    ADD COLUMN IF NOT EXISTS receiver_presave_id UUID REFERENCES receiver_presaves(id) ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_product_presaves_receiver
    ON product_presaves(receiver_presave_id)
    WHERE receiver_presave_id IS NOT NULL;

WITH unique_matches AS (
    SELECT p.id AS product_id, min(r.id::text)::uuid AS receiver_id
    FROM product_presaves p
    JOIN receiver_presaves r
      ON r.organization_id = p.organization_id
     AND r.deleted = false
     AND lower(btrim(r.organization_name)) = lower(btrim(p.original_manufacturer))
    WHERE p.receiver_presave_id IS NULL
      AND nullif(btrim(p.original_manufacturer), '') IS NOT NULL
    GROUP BY p.id
    HAVING count(*) = 1
)
UPDATE product_presaves p
SET receiver_presave_id = matches.receiver_id
FROM unique_matches matches
WHERE p.id = matches.product_id;
