ALTER TABLE drug_information
    ADD COLUMN IF NOT EXISTS source_product_id VARCHAR(255);

DO $$
BEGIN
    PERFORM set_config(
        'app.current_user_id',
        '00000000-0000-0000-0000-000000000001',
        true
    );
    PERFORM set_config(
        'app.current_organization_id',
        '00000000-0000-0000-0000-000000000000',
        true
    );
    PERFORM set_config('app.current_user_role', 'system_admin', true);
    PERFORM set_config('app.platform_isolation_bypass', 'true', true);

    UPDATE drug_information drug
     SET source_product_id = product.product_id
      FROM product_presaves product
     WHERE drug.source_product_presave_id = product.id
       AND product.product_id IS NOT NULL;
END
$$;
