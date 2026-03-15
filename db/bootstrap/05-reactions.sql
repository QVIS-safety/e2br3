-- ============================================================================
-- SECTION E: Reaction/Event (Repeating)
-- ============================================================================

CREATE TABLE reactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,  -- For ordering multiple reactions

    -- E.i.1.1 - Reaction/Event as Reported
    primary_source_reaction VARCHAR(250) NOT NULL,
    -- E.i.1.2 - Reaction/Event as Reported by Primary Source (translation)
    primary_source_reaction_translation VARCHAR(250),

    -- E.i.1.2 - Reaction/Event Language
    reaction_language VARCHAR(2),  -- ISO 639-1 code

    -- E.i.2.1 - MedDRA Coding (LLT or PT)
    reaction_meddra_version VARCHAR(10),  -- Version of MedDRA used
    reaction_meddra_code VARCHAR(20),      -- LLT or PT code

    -- E.i.3 - Term Highlighted by Reporter
    term_highlighted BOOLEAN,

    -- E.i.3.1 - Seriousness (MANDATORY if any seriousness criteria selected)
    serious BOOLEAN,

    -- E.i.3.2 - Seriousness Criteria (at least one if serious=true)
    criteria_death BOOLEAN NOT NULL DEFAULT FALSE,
    criteria_death_null_flavor VARCHAR(4) CHECK (criteria_death_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    criteria_life_threatening BOOLEAN NOT NULL DEFAULT FALSE,
    criteria_life_threatening_null_flavor VARCHAR(4) CHECK (criteria_life_threatening_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    criteria_hospitalization BOOLEAN NOT NULL DEFAULT FALSE,
    criteria_hospitalization_null_flavor VARCHAR(4) CHECK (criteria_hospitalization_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    criteria_disabling BOOLEAN NOT NULL DEFAULT FALSE,
    criteria_disabling_null_flavor VARCHAR(4) CHECK (criteria_disabling_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    criteria_congenital_anomaly BOOLEAN NOT NULL DEFAULT FALSE,
    criteria_congenital_anomaly_null_flavor VARCHAR(4) CHECK (criteria_congenital_anomaly_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    criteria_other_medically_important BOOLEAN NOT NULL DEFAULT FALSE,
    criteria_other_medically_important_null_flavor VARCHAR(4) CHECK (criteria_other_medically_important_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- FDA.E.i.3.2h - Required Intervention (FDA)
    required_intervention VARCHAR(10),

    -- E.i.4 - Date of Start of Reaction/Event
    start_date DATE,

    -- E.i.5 - Date of End of Reaction/Event
    end_date DATE,

    -- E.i.6 - Duration of Reaction/Event
    duration_value DECIMAL(10,2),
    duration_unit VARCHAR(3),  -- 800-805 codes

    -- E.i.7 - Outcome of Reaction/Event at Time of Last Observation
    outcome VARCHAR(1) CHECK (outcome IN ('0', '1', '2', '3', '4', '5')),
    -- 0=Unknown, 1=Recovered/resolved, 2=Recovering/resolving,
    -- 3=Not recovered/not resolved, 4=Recovered/resolved with sequelae, 5=Fatal

    -- E.i.8 - Medical Confirmation by Healthcare Professional
    medical_confirmation BOOLEAN,

    -- E.i.9 - Identification of Country Where Reaction/Event Occurred
    country_code VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- Null Flavor Support (E2B(R3) compliant: NI, UNK, ASKU, NASK, MSK)
    start_date_null_flavor VARCHAR(4) CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    end_date_null_flavor VARCHAR(4) CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    outcome_null_flavor VARCHAR(4) CHECK (outcome_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_reaction_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_reactions_case ON reactions(case_id);
CREATE INDEX idx_reactions_meddra ON reactions(reaction_meddra_code);
