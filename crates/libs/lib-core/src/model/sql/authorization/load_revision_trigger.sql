SELECT pg_get_triggerdef(t.oid),
       ARRAY(
           SELECT a.attname
           FROM unnest(t.tgattr::smallint[]) WITH ORDINALITY AS trigger_column(attnum, ordinal)
           JOIN pg_attribute a ON a.attrelid = t.tgrelid AND a.attnum = trigger_column.attnum
           ORDER BY trigger_column.ordinal
       ),
       t.tgtype,
       t.tgenabled::text,
       p.proname,
       p.prosecdef
FROM pg_trigger t
JOIN pg_class c ON c.oid = t.tgrelid
JOIN pg_namespace n ON n.oid = c.relnamespace
JOIN pg_proc p ON p.oid = t.tgfoid
WHERE n.nspname = current_schema()
  AND c.relname = $1
  AND t.tgname = $2
  AND NOT t.tgisinternal
