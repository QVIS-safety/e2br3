DO $$
DECLARE
    search_owner OID;
    search_owner_login BOOLEAN;
    search_owner_inherit BOOLEAN;
    search_owner_bypass BOOLEAN;
    search_owner_super BOOLEAN;
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
        WHERE rolname = current_user AND rolsuper
    ) THEN
        RAISE EXCEPTION
            'WHODrug search migration requires a superuser migration credential'
            USING ERRCODE = '42501';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_roles
        WHERE rolname = 'e2br3_whodrug_search_owner'
    ) THEN
        EXECUTE 'CREATE ROLE e2br3_whodrug_search_owner NOLOGIN NOINHERIT BYPASSRLS';
    END IF;

    SELECT oid, rolcanlogin, rolinherit, rolbypassrls, rolsuper
      INTO search_owner, search_owner_login, search_owner_inherit,
           search_owner_bypass, search_owner_super
      FROM pg_catalog.pg_roles
     WHERE rolname = 'e2br3_whodrug_search_owner';

    IF search_owner_login
       OR search_owner_inherit
       OR NOT (search_owner_bypass OR search_owner_super) THEN
        RAISE EXCEPTION
            'e2br3_whodrug_search_owner must be NOLOGIN, NOINHERIT, and BYPASSRLS or SUPERUSER'
            USING ERRCODE = '42501';
    END IF;

    IF EXISTS (
        SELECT 1 FROM pg_catalog.pg_auth_members
        WHERE roleid = search_owner OR member = search_owner
    ) THEN
        RAISE EXCEPTION
            'e2br3_whodrug_search_owner must have no role members or memberships'
            USING ERRCODE = '42501';
    END IF;
END;
$$;

REVOKE ALL ON SCHEMA public FROM e2br3_whodrug_search_owner;
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM e2br3_whodrug_search_owner;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM e2br3_whodrug_search_owner;
GRANT USAGE ON SCHEMA public TO e2br3_whodrug_search_owner;
GRANT SELECT ON TABLE public.whodrug_products, public.terminology_releases
    TO e2br3_whodrug_search_owner;

CREATE OR REPLACE FUNCTION public.search_active_whodrug_products(
    p_query TEXT,
    p_limit BIGINT
)
RETURNS TABLE (
    id BIGINT,
    code VARCHAR(20),
    drug_name TEXT,
    atc_code VARCHAR(20),
    version VARCHAR(20),
    language VARCHAR(2),
    active BOOLEAN,
    created_at TIMESTAMPTZ
)
LANGUAGE SQL
STABLE
STRICT
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
    SELECT
        products.id,
        products.code,
        products.drug_name,
        products.atc_code,
        products.version,
        products.language,
        TRUE,
        products.created_at
    FROM public.whodrug_products AS products
    JOIN public.terminology_releases AS releases
      ON releases.dictionary = 'whodrug'
     AND releases.version = products.version
     AND releases.language = products.language
    WHERE releases.status = 'active'
      AND products.drug_name ILIKE ('%' || p_query || '%')
    ORDER BY products.drug_name, products.code
    LIMIT LEAST(GREATEST(p_limit, 1), 100);
$$;

REVOKE ALL ON FUNCTION public.search_active_whodrug_products(TEXT, BIGINT)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.search_active_whodrug_products(TEXT, BIGINT)
    TO e2br3_app_role;
ALTER FUNCTION public.search_active_whodrug_products(TEXT, BIGINT)
    OWNER TO e2br3_whodrug_search_owner;
