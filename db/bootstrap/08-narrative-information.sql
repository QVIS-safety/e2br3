-- ============================================================================
-- SECTION H: Narrative and Other Information
-- ============================================================================

CREATE TABLE narrative_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,

    -- H.1 - Case Narrative Including Clinical Course, Therapeutic Measures, etc.
    case_narrative TEXT NOT NULL,  -- Free text, no length limit

    -- H.2 - Reporter's Comments
    reporter_comments TEXT,  -- Max 20000 chars

    -- H.4 - Sender's Comments
    sender_comments TEXT,  -- Max 2000 chars

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_narrative_per_case UNIQUE (case_id)
);

CREATE INDEX idx_narrative_case ON narrative_information(case_id);

-- ============================================================================
-- H.3.r: Sender's Diagnosis/Syndrome and/or Reclassification (Repeating, MedDRA)
-- ============================================================================

CREATE TABLE sender_diagnoses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    narrative_id UUID NOT NULL REFERENCES narrative_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- H.3.r.1 - Diagnosis/Syndrome (MedDRA coded)
    diagnosis_meddra_version VARCHAR(10),
    diagnosis_meddra_code VARCHAR(20),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_diagnosis_sequence UNIQUE (narrative_id, sequence_number)
);

CREATE INDEX idx_sender_diagnoses ON sender_diagnoses(narrative_id);

-- ============================================================================
-- H.5.r: Case Summary and Further Information
-- ============================================================================

CREATE TABLE case_summary_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    narrative_id UUID NOT NULL REFERENCES narrative_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- H.5.r.1 - Case Summary Type
    summary_type VARCHAR(2),  -- E2B(R3) code list

    -- H.5.r.2 - Case Summary Language
    language_code VARCHAR(2),  -- ISO 639-1

    -- H.5.r.3 - Text
    summary_text TEXT,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_case_summary_sequence UNIQUE (narrative_id, sequence_number)
);

CREATE INDEX idx_case_summary ON case_summary_information(narrative_id);