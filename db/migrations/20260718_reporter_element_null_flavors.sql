ALTER TABLE primary_sources
    DROP COLUMN IF EXISTS reporter_name_null_flavor,
    DROP COLUMN IF EXISTS reporter_address_null_flavor,
    ADD COLUMN IF NOT EXISTS reporter_title_null_flavor VARCHAR(4)
        CHECK (reporter_title_null_flavor IN ('MSK', 'UNK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS reporter_given_name_null_flavor VARCHAR(4)
        CHECK (reporter_given_name_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS reporter_middle_name_null_flavor VARCHAR(4)
        CHECK (reporter_middle_name_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS reporter_family_name_null_flavor VARCHAR(4)
        CHECK (reporter_family_name_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS organization_null_flavor VARCHAR(4)
        CHECK (organization_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS department_null_flavor VARCHAR(4)
        CHECK (department_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS street_null_flavor VARCHAR(4)
        CHECK (street_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS city_null_flavor VARCHAR(4)
        CHECK (city_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS state_null_flavor VARCHAR(4)
        CHECK (state_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS postcode_null_flavor VARCHAR(4)
        CHECK (postcode_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS telephone_null_flavor VARCHAR(4)
        CHECK (telephone_null_flavor IN ('MSK', 'ASKU', 'NASK'));

ALTER TABLE reporter_presaves
    DROP COLUMN IF EXISTS reporter_name_null_flavor,
    DROP COLUMN IF EXISTS reporter_address_null_flavor,
    ADD COLUMN IF NOT EXISTS reporter_title_null_flavor VARCHAR(4)
        CHECK (reporter_title_null_flavor IN ('MSK', 'UNK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS reporter_given_name_null_flavor VARCHAR(4)
        CHECK (reporter_given_name_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS reporter_middle_name_null_flavor VARCHAR(4)
        CHECK (reporter_middle_name_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS reporter_family_name_null_flavor VARCHAR(4)
        CHECK (reporter_family_name_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS organization_null_flavor VARCHAR(4)
        CHECK (organization_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS department_null_flavor VARCHAR(4)
        CHECK (department_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS street_null_flavor VARCHAR(4)
        CHECK (street_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS city_null_flavor VARCHAR(4)
        CHECK (city_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS state_null_flavor VARCHAR(4)
        CHECK (state_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS postcode_null_flavor VARCHAR(4)
        CHECK (postcode_null_flavor IN ('MSK', 'ASKU', 'NASK')),
    ADD COLUMN IF NOT EXISTS telephone_null_flavor VARCHAR(4)
        CHECK (telephone_null_flavor IN ('MSK', 'ASKU', 'NASK'));
