-- Preserve existing hashes as v1; all new rows use an unambiguous v2 payload.
ALTER TABLE audit_logs ADD COLUMN hash_version SMALLINT NOT NULL DEFAULT 1;
ALTER TABLE audit_logs ALTER COLUMN hash_version SET DEFAULT 2;
-- Existing rows receive the metadata-only constant default 1, while NOT VALID
-- still enforces every new write. Validate separately after this migration commits
-- so the historical-table scan does not hold the migration's write-blocking lock.
ALTER TABLE audit_logs ADD CONSTRAINT audit_logs_hash_version_valid
    CHECK (hash_version IN (1, 2)) NOT VALID;

CREATE OR REPLACE FUNCTION audit_logs_hash_chain_before_insert()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public, pg_temp
AS $$
DECLARE
    v_prev_hash TEXT;
    v_payload TEXT;
BEGIN
    IF NEW.created_at IS NULL THEN
        RAISE EXCEPTION 'audit created_at must not be null'
            USING ERRCODE = '23502';
    END IF;

    PERFORM pg_advisory_xact_lock(
        hashtextextended('e2br3.audit_logs.hash_chain', 0)
    );

    SELECT entry_hash INTO v_prev_hash
      FROM public.audit_logs
     ORDER BY id DESC
     LIMIT 1;

    IF NOT FOUND THEN
        v_prev_hash := repeat('0', 64);
    ELSIF v_prev_hash IS NULL OR v_prev_hash !~ '^[0-9A-Fa-f]{64}$' THEN
        RAISE EXCEPTION 'audit chain head hash is invalid'
            USING ERRCODE = '55000';
    END IF;
    NEW.prev_hash := v_prev_hash;
    NEW.hash_version := 2;
    v_payload := jsonb_build_array(
        NEW.hash_version,
        NEW.id,
        NEW.prev_hash,
        NEW.organization_id,
        NEW.table_name,
        NEW.record_id,
        NEW.action,
        NEW.user_id,
        NEW.reason_for_change,
        NEW.change_category,
        NEW.e_signature_id,
        jsonb_build_array(NEW.old_values IS NULL, NEW.old_values),
        jsonb_build_array(NEW.new_values IS NULL, NEW.new_values),
        jsonb_build_array(NEW.changed_fields IS NULL, NEW.changed_fields),
        NEW.ip_address::TEXT,
        NEW.user_agent,
        to_char(NEW.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
    )::TEXT;
    NEW.entry_hash := encode(public.digest(v_payload, 'sha256'), 'hex');
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION prevent_audit_log_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'audit logs are append-only' USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER audit_logs_reject_update_delete
    BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_log_mutation();
CREATE TRIGGER audit_logs_reject_truncate
    BEFORE TRUNCATE ON audit_logs
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_audit_log_mutation();

REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON audit_logs FROM e2br3_app_role;

CREATE OR REPLACE FUNCTION verify_audit_log_hash_chain_range(
    p_since_id BIGINT DEFAULT NULL,
    p_until_id BIGINT DEFAULT NULL,
    p_limit INTEGER DEFAULT 1000
)
RETURNS TABLE (
    total_rows BIGINT,
    verified_ok_rows BIGINT,
    broken_rows BIGINT,
    first_broken_id BIGINT,
    first_broken_reason TEXT,
    checked_at TIMESTAMPTZ,
    checked_from_id BIGINT,
    checked_through_id BIGINT,
    snapshot_head_id BIGINT,
    snapshot_head_hash TEXT,
    next_since_id BIGINT,
    has_more BOOLEAN,
    legacy_rows BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_snapshot_head_id BIGINT;
    v_snapshot_head_hash TEXT;
BEGIN
    IF p_limit IS NULL OR p_limit < 1 OR p_limit > 10000 THEN
        RAISE EXCEPTION 'audit verification limit must be between 1 and 10000'
            USING ERRCODE = '22023';
    END IF;
    IF p_since_id IS NOT NULL AND p_since_id < 0 THEN
        RAISE EXCEPTION 'audit verification since id must be non-negative'
            USING ERRCODE = '22023';
    END IF;
    IF p_until_id IS NOT NULL AND p_until_id < 0 THEN
        RAISE EXCEPTION 'audit verification until id must be non-negative'
            USING ERRCODE = '22023';
    END IF;
    IF p_since_id IS NOT NULL AND p_until_id IS NOT NULL
       AND p_since_id > p_until_id THEN
        RAISE EXCEPTION 'audit verification since id must not exceed until id'
            USING ERRCODE = '22023';
    END IF;
    IF p_until_id IS NULL THEN
        SELECT a.id, a.entry_hash::TEXT
          INTO v_snapshot_head_id, v_snapshot_head_hash
          FROM public.audit_logs a
         ORDER BY a.id DESC LIMIT 1;
    ELSE
        SELECT a.id, a.entry_hash::TEXT
          INTO v_snapshot_head_id, v_snapshot_head_hash
          FROM public.audit_logs a
         WHERE a.id = p_until_id;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'audit verification until id does not exist'
                USING ERRCODE = '22023';
        END IF;
    END IF;

    RETURN QUERY
    WITH snapshot AS MATERIALIZED (
        SELECT v_snapshot_head_id AS id, v_snapshot_head_hash AS entry_hash
         WHERE v_snapshot_head_id IS NOT NULL
    ),
    -- Scalar bounds stay PK index conditions; cross joins can become late join
    -- filters and scan millions of audit rows before applying the page boundary.
    page AS MATERIALIZED (
        SELECT a.*
          FROM public.audit_logs a
         WHERE a.id >= coalesce(p_since_id, '-9223372036854775808'::BIGINT)
           AND a.id <= (SELECT id FROM snapshot)
         ORDER BY a.id
         LIMIT p_limit + 1
    ),
    bounded AS MATERIALIZED (
        SELECT * FROM page ORDER BY id LIMIT p_limit
    ),
    first_row AS (
        SELECT min(id) AS id FROM bounded
    ),
    predecessor AS (
        SELECT a.id, a.entry_hash::TEXT
          FROM public.audit_logs a
         WHERE a.id < (SELECT id FROM first_row)
         ORDER BY a.id DESC LIMIT 1
    ),
    checked AS (
        SELECT b.id,
               b.hash_version,
               CASE
                   WHEN b.hash_version IS NULL OR b.hash_version NOT IN (1, 2)
                       THEN 'unknown hash version'
                   WHEN b.prev_hash IS NULL OR b.prev_hash !~ '^[0-9A-Fa-f]{64}$'
                       THEN 'prev_hash is not a 64-char hex value'
                   WHEN b.entry_hash IS NULL OR b.entry_hash !~ '^[0-9A-Fa-f]{64}$'
                       THEN 'entry_hash is not a 64-char hex value'
                   WHEN (SELECT id FROM predecessor) IS NOT NULL
                        AND b.id = (SELECT id FROM first_row)
                        AND ((SELECT entry_hash FROM predecessor) IS NULL
                             OR (SELECT entry_hash FROM predecessor) !~ '^[0-9A-Fa-f]{64}$')
                       THEN 'previous entry_hash is not a 64-char hex value'
                   WHEN b.prev_hash::TEXT <> coalesce(
                       lag(b.entry_hash::TEXT) OVER (ORDER BY b.id),
                       (SELECT entry_hash FROM predecessor),
                       repeat('0', 64)
                   ) THEN 'prev_hash does not match previous entry_hash'
                   WHEN b.entry_hash::TEXT <> CASE b.hash_version
                       WHEN 1 THEN encode(public.digest(concat_ws('|',
                           coalesce(b.id::TEXT, ''), coalesce(b.prev_hash::TEXT, ''),
                           b.table_name, b.record_id::TEXT, b.action, b.user_id::TEXT,
                           coalesce(b.reason_for_change, ''), coalesce(b.change_category, ''),
                           coalesce(b.e_signature_id::TEXT, ''), coalesce(b.old_values::TEXT, 'null'),
                           coalesce(b.new_values::TEXT, 'null'), coalesce(b.changed_fields::TEXT, 'null'),
                           coalesce(b.ip_address::TEXT, ''), coalesce(b.user_agent, ''),
                           to_char(b.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                       ), 'sha256'), 'hex')
                       WHEN 2 THEN encode(public.digest(jsonb_build_array(
                           b.hash_version, b.id, b.prev_hash::TEXT, b.organization_id,
                           b.table_name, b.record_id, b.action, b.user_id,
                           b.reason_for_change, b.change_category, b.e_signature_id,
                           jsonb_build_array(b.old_values IS NULL, b.old_values),
                           jsonb_build_array(b.new_values IS NULL, b.new_values),
                           jsonb_build_array(b.changed_fields IS NULL, b.changed_fields),
                           b.ip_address::TEXT, b.user_agent,
                           to_char(b.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')
                       )::TEXT, 'sha256'), 'hex')
                   END THEN 'entry_hash does not match recomputed payload hash'
               END AS reason
          FROM bounded b
    ),
    summary AS (
        SELECT count(*)::BIGINT AS total_rows,
               count(*) FILTER (WHERE reason IS NULL)::BIGINT AS ok_rows,
               count(*) FILTER (WHERE reason IS NOT NULL)::BIGINT AS broken_rows,
               min(id) FILTER (WHERE reason IS NOT NULL) AS first_broken_id,
               (array_agg(reason ORDER BY id) FILTER (WHERE reason IS NOT NULL))[1] AS first_reason,
               min(id) AS checked_from_id,
               max(id) AS checked_through_id,
               count(*) FILTER (WHERE hash_version = 1)::BIGINT AS legacy_rows
          FROM checked
    )
    SELECT x.total_rows, x.ok_rows, x.broken_rows, x.first_broken_id,
           x.first_reason, clock_timestamp(), x.checked_from_id,
           x.checked_through_id, s.id, s.entry_hash,
           (SELECT id FROM page ORDER BY id OFFSET p_limit LIMIT 1),
           EXISTS (SELECT 1 FROM page OFFSET p_limit), x.legacy_rows
      FROM summary x
      LEFT JOIN snapshot s ON TRUE;
END;
$$;

REVOKE ALL ON FUNCTION verify_audit_log_hash_chain_range(BIGINT, BIGINT, INTEGER)
    FROM PUBLIC;
GRANT EXECUTE ON FUNCTION verify_audit_log_hash_chain_range(BIGINT, BIGINT, INTEGER)
    TO e2br3_app_role;

-- Keep the legacy result shape for callers while fixing range-boundary checking.
CREATE OR REPLACE FUNCTION verify_audit_log_hash_chain(p_since_id BIGINT DEFAULT NULL)
RETURNS TABLE (
    total_rows BIGINT,
    verified_ok_rows BIGINT,
    broken_rows BIGINT,
    first_broken_id BIGINT,
    first_broken_reason TEXT,
    checked_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_head_id BIGINT;
    v_next BIGINT := p_since_id;
    v_page RECORD;
BEGIN
    total_rows := 0;
    verified_ok_rows := 0;
    broken_rows := 0;
    first_broken_id := NULL;
    first_broken_reason := NULL;
    checked_at := clock_timestamp();

    SELECT max(a.id) INTO v_head_id FROM public.audit_logs a;
    IF v_head_id IS NULL THEN
        RETURN NEXT;
        RETURN;
    END IF;
    IF p_since_id IS NOT NULL AND p_since_id > v_head_id THEN
        RETURN NEXT;
        RETURN;
    END IF;
    IF p_since_id IS NOT NULL AND p_since_id < 0 THEN
        v_next := NULL;
    END IF;

    LOOP
        SELECT * INTO v_page
          FROM public.verify_audit_log_hash_chain_range(v_next, v_head_id, 10000);
        total_rows := total_rows + v_page.total_rows;
        verified_ok_rows := verified_ok_rows + v_page.verified_ok_rows;
        broken_rows := broken_rows + v_page.broken_rows;
        IF first_broken_id IS NULL AND v_page.first_broken_id IS NOT NULL THEN
            first_broken_id := v_page.first_broken_id;
            first_broken_reason := v_page.first_broken_reason;
        END IF;
        EXIT WHEN NOT v_page.has_more;
        v_next := v_page.next_since_id;
    END LOOP;
    RETURN NEXT;
END;
$$;

REVOKE ALL ON FUNCTION verify_audit_log_hash_chain(BIGINT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION verify_audit_log_hash_chain(BIGINT) TO e2br3_app_role;
