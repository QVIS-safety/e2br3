-- MedDRA uses the existing audited release lifecycle, independently per language.
-- Legacy row active flags no longer define visibility; do not rewrite vocabulary rows.
CREATE UNIQUE INDEX IF NOT EXISTS meddra_one_active_release ON terminology_releases(dictionary,language)
    WHERE dictionary='meddra' AND status='active';
DROP POLICY IF EXISTS meddra_terms_read ON meddra_terms;
CREATE POLICY meddra_terms_read ON meddra_terms FOR SELECT TO e2br3_app_role
    USING (is_current_user_admin() OR EXISTS (
        SELECT 1 FROM terminology_releases r WHERE r.dictionary='meddra'
        AND r.version=meddra_terms.version AND r.language=meddra_terms.language AND r.status='active'
    ));
CREATE INDEX IF NOT EXISTS idx_meddra_release ON meddra_terms(version,language);
