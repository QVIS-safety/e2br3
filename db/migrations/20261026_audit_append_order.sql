-- Acquire the chain lock before BIGSERIAL assigns IDs. A row trigger is too
-- late: a waiting writer can reserve an ID between two rows of another transaction.
CREATE FUNCTION lock_audit_log_append() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('e2br3.audit_logs.hash_chain', 0));
    RETURN NULL;
END;
$$;

CREATE TRIGGER audit_logs_lock_before_statement
    BEFORE INSERT ON audit_logs
    FOR EACH STATEMENT EXECUTE FUNCTION lock_audit_log_append();
