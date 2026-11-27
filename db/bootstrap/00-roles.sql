\if :{?app_db_user}
\else
\echo 'app_db_user psql variable is required'
\quit 1
\endif

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Create database roles before any table grants/policies reference them.
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'e2br3_app_role') THEN
        CREATE ROLE e2br3_app_role;
    END IF;
END $$;

GRANT e2br3_app_role TO :"app_db_user";

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'e2br3_auditor_role') THEN
        CREATE ROLE e2br3_auditor_role;
    END IF;
END $$;


GRANT e2br3_auditor_role TO :"app_db_user";
CREATE EXTENSION IF NOT EXISTS pg_trgm;
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
            'WHODrug search initialization requires a superuser migration credential'
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
