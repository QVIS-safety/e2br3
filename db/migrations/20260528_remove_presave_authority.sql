DROP INDEX IF EXISTS idx_sender_presaves_authority;
DROP INDEX IF EXISTS idx_receiver_presaves_authority;
DROP INDEX IF EXISTS idx_product_presaves_authority;
DROP INDEX IF EXISTS idx_reporter_presaves_authority;
DROP INDEX IF EXISTS idx_study_presaves_authority;
DROP INDEX IF EXISTS idx_narrative_presaves_authority;

ALTER TABLE sender_presaves
    DROP CONSTRAINT IF EXISTS sender_presaves_authority_valid,
    DROP COLUMN IF EXISTS authority;

ALTER TABLE receiver_presaves
    DROP CONSTRAINT IF EXISTS receiver_presaves_authority_valid,
    DROP COLUMN IF EXISTS authority;

ALTER TABLE product_presaves
    DROP CONSTRAINT IF EXISTS product_presaves_authority_valid,
    DROP COLUMN IF EXISTS authority;

ALTER TABLE reporter_presaves
    DROP CONSTRAINT IF EXISTS reporter_presaves_authority_valid,
    DROP COLUMN IF EXISTS authority;

ALTER TABLE study_presaves
    DROP CONSTRAINT IF EXISTS study_presaves_authority_valid,
    DROP COLUMN IF EXISTS authority;

ALTER TABLE narrative_presaves
    DROP CONSTRAINT IF EXISTS narrative_presaves_authority_valid,
    DROP COLUMN IF EXISTS authority;
