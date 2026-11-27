-- Excludes only initializer/UI bookkeeping and environment-specific login grants.
-- Definitions, RLS flags, function security settings, and canonical ACLs remain.
WITH objects AS (
    SELECT 'relation' AS kind, c.relname::text AS name,
           jsonb_build_object('kind', c.relkind, 'rls', c.relrowsecurity,
                              'forceRls', c.relforcerowsecurity) AS definition
    FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
    WHERE n.nspname='public' AND c.relkind IN ('r','p','v','m','S')
      AND c.relname NOT IN ('schema_migrations','schema_baseline','ui_env_ready')
    UNION ALL
    SELECT 'column', c.relname || '.' || a.attname,
           jsonb_build_object('type', format_type(a.atttypid,a.atttypmod),
             'nullable', NOT a.attnotnull, 'identity', a.attidentity,
             'generated', a.attgenerated, 'default', pg_get_expr(d.adbin,d.adrelid))
    FROM pg_attribute a JOIN pg_class c ON c.oid=a.attrelid
    JOIN pg_namespace n ON n.oid=c.relnamespace
    LEFT JOIN pg_attrdef d ON d.adrelid=a.attrelid AND d.adnum=a.attnum
    WHERE n.nspname='public' AND c.relkind IN ('r','p','v','m')
      AND a.attnum>0 AND NOT a.attisdropped
      AND c.relname NOT IN ('schema_migrations','schema_baseline','ui_env_ready')
    UNION ALL
    SELECT 'constraint', c.relname || '.' || k.conname,
           to_jsonb(pg_get_constraintdef(k.oid))
    FROM pg_constraint k JOIN pg_class c ON c.oid=k.conrelid
    WHERE k.connamespace='public'::regnamespace
      AND c.relname NOT IN ('schema_migrations','schema_baseline','ui_env_ready')
    UNION ALL
    SELECT 'index', indexname, to_jsonb(indexdef)
    FROM pg_indexes WHERE schemaname='public'
      AND tablename NOT IN ('schema_migrations','schema_baseline','ui_env_ready')
    UNION ALL
    SELECT 'trigger', c.relname || '.' || t.tgname,
           jsonb_build_object('definition', pg_get_triggerdef(t.oid), 'enabled', t.tgenabled)
    FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid
    WHERE c.relnamespace='public'::regnamespace AND NOT t.tgisinternal
    UNION ALL
    SELECT 'function', p.proname || '(' || pg_get_function_identity_arguments(p.oid) || ')',
           to_jsonb(pg_get_functiondef(p.oid))
    FROM pg_proc p WHERE p.pronamespace='public'::regnamespace
      AND NOT EXISTS (SELECT FROM pg_depend d WHERE d.classid='pg_proc'::regclass AND d.objid=p.oid AND d.deptype='e')
    UNION ALL
    SELECT 'policy', tablename || '.' || policyname,
           jsonb_build_object('roles', roles, 'command', cmd, 'permissive', permissive, 'using', qual, 'check', with_check)
    FROM pg_policies WHERE schemaname='public'
    UNION ALL
    SELECT 'sequence', sequencename,
           jsonb_build_object('type', data_type, 'start', start_value::text, 'min', min_value::text,
                             'max', max_value::text, 'increment', increment_by::text, 'cycle', cycle, 'cache', cache_size::text)
    FROM pg_sequences WHERE schemaname='public'
    UNION ALL
    SELECT 'extension', extname, to_jsonb(extversion) FROM pg_extension
    UNION ALL
    SELECT 'relation_acl', c.relname || ':' || r.rolname,
           jsonb_agg(jsonb_build_object('privilege', a.privilege_type, 'grantable', a.is_grantable) ORDER BY a.privilege_type)
    FROM pg_class c CROSS JOIN LATERAL aclexplode(c.relacl) a
    JOIN pg_roles r ON r.oid=a.grantee
    WHERE c.relnamespace='public'::regnamespace
      AND r.rolname IN ('e2br3_app_role','e2br3_auditor_role','e2br3_whodrug_search_owner')
    GROUP BY c.relname, r.rolname
    UNION ALL
    SELECT 'function_acl', p.proname || '(' || pg_get_function_identity_arguments(p.oid) || '):' || r.rolname,
           jsonb_agg(jsonb_build_object('privilege', a.privilege_type, 'grantable', a.is_grantable) ORDER BY a.privilege_type)
    FROM pg_proc p CROSS JOIN LATERAL aclexplode(p.proacl) a
    JOIN pg_roles r ON r.oid=a.grantee
    WHERE p.pronamespace='public'::regnamespace
      AND r.rolname IN ('e2br3_app_role','e2br3_auditor_role','e2br3_whodrug_search_owner')
    GROUP BY p.oid, r.rolname
)
SELECT jsonb_object_agg(kind || ':' || name, definition ORDER BY kind,name) FROM objects;
