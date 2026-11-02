-- Stream a case audit graph without materializing the full audit log table.
-- SECURITY INVOKER preserves the caller's audit_logs RLS visibility.
CREATE OR REPLACE FUNCTION public.list_case_audit_logs(p_case_id UUID)
RETURNS SETOF public.audit_logs
LANGUAGE plpgsql
STABLE
SECURITY INVOKER
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    v_organization_id UUID;
    v_seen UUID[];
    v_frontier UUID[];
    v_next_frontier UUID[];
    v_frontier_chunk UUID[];
    v_children UUID[];
    v_offset INTEGER;
BEGIN
    FOR v_organization_id IN
        SELECT DISTINCT logs.organization_id
          FROM public.audit_logs AS logs
         WHERE logs.table_name = 'cases'
           AND logs.record_id = p_case_id
    LOOP
        v_seen := ARRAY[p_case_id];
        v_frontier := ARRAY[p_case_id];

        WHILE pg_catalog.cardinality(v_frontier) > 0 LOOP
            v_next_frontier := ARRAY[]::UUID[];
            v_offset := 1;

            WHILE v_offset <= pg_catalog.cardinality(v_frontier) LOOP
                v_frontier_chunk := v_frontier[v_offset:v_offset + 999];

                SELECT pg_catalog.array_agg(DISTINCT logs.record_id)
                  INTO v_children
                  FROM public.audit_logs AS logs
                 WHERE logs.organization_id = v_organization_id
                   AND logs.parent_record_ids && v_frontier_chunk
                   AND NOT (logs.record_id = ANY(v_seen))
                   AND NOT (logs.record_id = ANY(v_next_frontier));

                IF v_children IS NULL THEN
                    v_offset := v_offset + 1000;
                    CONTINUE;
                END IF;

                v_next_frontier := v_next_frontier || v_children;
                v_offset := v_offset + 1000;
            END LOOP;

            IF pg_catalog.cardinality(v_next_frontier) = 0 THEN
                EXIT;
            END IF;

            v_seen := v_seen || v_next_frontier;
            v_frontier := v_next_frontier;
        END LOOP;

        RETURN QUERY
        SELECT logs.*
          FROM public.audit_logs AS logs
         WHERE logs.organization_id = v_organization_id
           AND logs.record_id = ANY(v_seen);
    END LOOP;
END;
$$;

REVOKE ALL ON FUNCTION public.list_case_audit_logs(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.list_case_audit_logs(UUID) TO e2br3_app_role;
