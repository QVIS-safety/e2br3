-- Read-only Oracle snapshot for the derived MFDS XML fixture. Output is private.
BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;
SET LOCAL statement_timeout = '15s';
WITH releases AS (
    SELECT dictionary, version, language, status, source_checksum, loaded_rows
    FROM terminology_releases
    WHERE status = 'active' AND (
        (dictionary = 'meddra' AND version = '28.1' AND language = 'en') OR
        (dictionary = 'mfds_product' AND version = '2026-10-01' AND language = 'ko') OR
        (dictionary = 'ich_constrained_ucum' AND version = '2.2' AND language = 'en')
    )
), meddra AS (
    SELECT t.* FROM meddra_terms t JOIN releases r
    ON r.dictionary = 'meddra' AND r.version = t.version AND r.language = t.language
    WHERE t.active AND t.level = 'LLT'
    AND t.code IN ('10006824', '10020603', '10020772', '10022617', '10040047', '10062060')
), substances AS (
    SELECT DISTINCT ON (t.substance_code) t.*
    FROM mfds_product_substances t JOIN releases r
    ON r.dictionary = 'mfds_product' AND r.version = t.version
    WHERE t.active AND t.substance_code IN ('M084105', 'M222875')
    ORDER BY t.substance_code, (t.item_seq IN ('201904647', '200200143')) DESC,
        t.item_seq, t.material_sequence, t.total_amount_sequence
), products AS (
    SELECT t.* FROM mfds_products t JOIN releases r
    ON r.dictionary = 'mfds_product' AND r.version = t.version
    WHERE t.active AND (t.item_seq IN ('201904647', '200200143')
        OR t.item_seq IN (SELECT item_seq FROM substances))
), ucum AS (
    SELECT t.* FROM controlled_terminology_terms t JOIN releases r
    ON r.dictionary = t.dictionary AND r.version = t.version AND r.language = t.language
    WHERE t.active AND t.dictionary = 'ich_constrained_ucum' AND t.scope = 'frequency' AND t.code = 'd'
)
SELECT jsonb_build_object(
    'source', current_database(), 'captured_at', transaction_timestamp(),
    'source_releases', (SELECT jsonb_agg(to_jsonb(r) ORDER BY dictionary) FROM releases r),
    'tables', jsonb_build_object(
        'meddra_terms', (SELECT jsonb_agg(to_jsonb(t) - ARRAY['id', 'audit_id', 'created_at'] ORDER BY code) FROM meddra t),
        'mfds_products', (SELECT jsonb_agg(to_jsonb(t) - ARRAY['id', 'audit_id', 'created_at'] ORDER BY item_seq) FROM products t),
        'mfds_product_substances', (SELECT jsonb_agg(to_jsonb(t) - ARRAY['id', 'audit_id', 'created_at'] ORDER BY substance_code) FROM substances t),
        'controlled_terminology_terms', (SELECT jsonb_agg(to_jsonb(t) - ARRAY['id', 'audit_id', 'created_at']) FROM ucum t)
    )
);
COMMIT;
