-- WHODrug activation changes release metadata, never millions of product/CAS rows.
-- Keep old row flags as historical data; all WHODrug readers use release status.
ALTER TABLE terminology_releases ADD COLUMN IF NOT EXISTS audit_id UUID NOT NULL DEFAULT gen_random_uuid();
CREATE UNIQUE INDEX IF NOT EXISTS terminology_releases_audit_id_key ON terminology_releases(audit_id);
CREATE UNIQUE INDEX IF NOT EXISTS whodrug_one_active_release ON terminology_releases(dictionary, language)
    WHERE dictionary = 'whodrug' AND status = 'active';
DROP TRIGGER IF EXISTS audit_terminology_releases ON terminology_releases;
CREATE TRIGGER audit_terminology_releases AFTER INSERT OR UPDATE OR DELETE ON terminology_releases
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function_with_audit_id();
CREATE INDEX IF NOT EXISTS idx_whodrug_release ON whodrug_products(version, language);
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_whodrug_search_trgm ON whodrug_products USING gin(drug_name gin_trgm_ops);

-- Preserve the active-only reader boundary using the same release as search/validation.
DROP POLICY IF EXISTS whodrug_products_read ON whodrug_products;
CREATE POLICY whodrug_products_read ON whodrug_products FOR SELECT TO e2br3_app_role
    USING (is_current_user_admin() OR EXISTS (
        SELECT 1 FROM terminology_releases r WHERE r.dictionary='whodrug'
        AND r.version=whodrug_products.version AND r.language=whodrug_products.language AND r.status='active'
    ));
DROP POLICY IF EXISTS controlled_terminology_terms_read ON controlled_terminology_terms;
CREATE POLICY controlled_terminology_terms_read ON controlled_terminology_terms FOR SELECT TO e2br3_app_role
    USING (is_current_user_admin() OR (dictionary <> 'whodrug' AND active=true)
        OR (dictionary='whodrug' AND EXISTS (
            SELECT 1 FROM terminology_releases r WHERE r.dictionary='whodrug'
            AND r.version=controlled_terminology_terms.version
            AND r.language=controlled_terminology_terms.language AND r.status='active'
        )));
