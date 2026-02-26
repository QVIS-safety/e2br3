-- ============================================================================
-- Terminology Release Tracking (MedDRA / WHODrug)
-- ============================================================================

CREATE TABLE IF NOT EXISTS terminology_releases (
    id BIGSERIAL PRIMARY KEY,
    dictionary VARCHAR(20) NOT NULL,
    version VARCHAR(40) NOT NULL,
    language VARCHAR(10) NOT NULL DEFAULT 'en',
    status VARCHAR(20) NOT NULL DEFAULT 'loading',
    source_path TEXT,
    source_checksum VARCHAR(128),
    loaded_rows BIGINT NOT NULL DEFAULT 0,
    approved_by UUID,
    approved_at TIMESTAMPTZ,
    activated_by UUID,
    activated_at TIMESTAMPTZ,
    rollback_from_version VARCHAR(40),
    note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT terminology_release_dictionary_chk
        CHECK (dictionary IN ('meddra', 'whodrug')),
    CONSTRAINT terminology_release_status_chk
        CHECK (status IN ('loading', 'validated', 'approved', 'active', 'failed', 'retired')),
    CONSTRAINT terminology_release_unique
        UNIQUE (dictionary, version, language)
);

CREATE INDEX IF NOT EXISTS idx_terminology_releases_lookup
    ON terminology_releases (dictionary, language, status, activated_at DESC);
