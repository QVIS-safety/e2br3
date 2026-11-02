SELECT EXISTS (
    SELECT 1
    FROM pg_trigger t
    JOIN pg_class c ON c.oid = t.tgrelid
    JOIN pg_namespace n ON n.oid = c.relnamespace
    JOIN pg_proc p ON p.oid = t.tgfoid
    WHERE n.nspname = current_schema()
      AND c.relname = $1
      AND t.tgname = $2
      AND NOT t.tgisinternal
      AND t.tgenabled = 'O'
      AND p.proname = $2
      AND p.prosecdef
      AND (t.tgtype & 1) = 1
      AND (t.tgtype & 4) = 4
)
