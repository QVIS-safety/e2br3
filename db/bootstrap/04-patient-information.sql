-- ============================================================================
-- SECTION D: Patient Information
-- ============================================================================

CREATE TABLE patient_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,

    -- D.1 - Patient (name identifier)
    patient_initials VARCHAR(60),
    patient_given_name VARCHAR(60),  -- Can use null flavor
    patient_family_name VARCHAR(60),  -- Can use null flavor

    -- D.2.1 - Date of Birth
    birth_date DATE,  -- Can use null flavor

    -- D.2.2 - Age Information
    age_at_time_of_onset DECIMAL(5,2),
    age_unit VARCHAR(3),  -- E2B(R3) codes: 800-805 (year/month/week/day/hour/decade)

    -- D.2.2.1 - Gestation Period (for fetal cases)
    gestation_period DECIMAL(5,2),
    gestation_period_unit VARCHAR(3),  -- 802, 803, 804 (month, week, day)

    -- D.2.3 - Patient Age Group (E2B(R3) codes)
    age_group VARCHAR(1) CHECK (age_group IN ('1', '2', '3', '4', '5', '6')),
    -- 1=Neonate, 2=Infant, 3=Child, 4=Adolescent, 5=Adult, 6=Elderly

    -- D.3 - Body Weight (kg)
    weight_kg DECIMAL(6,2),

    -- D.4 - Height (cm)
    height_cm DECIMAL(6,2),

    -- D.5 - Sex (E2B(R3) codes)
    sex VARCHAR(1) CHECK (sex IN ('0', '1', '2')),  -- 0=Unknown, 1=Male, 2=Female

    -- FDA.D.11 / FDA.D.12 - Race / Ethnicity (FDA)
    race_code VARCHAR(10),
    ethnicity_code VARCHAR(10),

    -- D.6 - Last Menstrual Period Date
    last_menstrual_period_date DATE,
    last_menstrual_period_date_null_flavor VARCHAR(4) CHECK (last_menstrual_period_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.7.2 - Text for Relevant Medical History
    medical_history_text TEXT,  -- Max 10000 chars

    -- D.7.3 - Concomitant Therapies
    concomitant_therapy BOOLEAN,

    -- Null Flavor Support (E2B(R3) compliant: NI, UNK, ASKU, NASK, MSK)
    patient_initials_null_flavor VARCHAR(4) CHECK (patient_initials_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    birth_date_null_flavor VARCHAR(4) CHECK (birth_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    age_at_time_of_onset_null_flavor VARCHAR(4) CHECK (age_at_time_of_onset_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    sex_null_flavor VARCHAR(4) CHECK (sex_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_patient_per_case UNIQUE (case_id)
);

CREATE INDEX idx_patient_info_case ON patient_information(case_id);

-- ============================================================================
-- D.1.1: Patient Medical Record Number(s) and Source(s) of the Record Number (Repeating)
-- ============================================================================

CREATE TABLE patient_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patient_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- D.1.1.x - Patient Medical Record Number(s) and Source(s) of the Record Number
    identifier_type_code VARCHAR(1) NOT NULL, -- 1=GP, 2=Specialist, 3=Hospital, 4=Investigation
    identifier_value VARCHAR(100) NOT NULL,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_patient_identifier_sequence UNIQUE (patient_id, sequence_number)
);

CREATE INDEX idx_patient_identifiers_patient ON patient_identifiers(patient_id);

-- ============================================================================
-- D.7.1.r: Medical History Episodes (Repeating)
-- ============================================================================

CREATE TABLE medical_history_episodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patient_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- D.7.1.r.1a - Disease/Surgical Procedure/etc (MedDRA coded)
    meddra_version VARCHAR(10),
    meddra_code VARCHAR(20),

    -- D.7.1.r.2 - Start Date
    start_date DATE,
    start_date_null_flavor VARCHAR(4) CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.7.1.r.3 - Continuing
    continuing BOOLEAN,

    -- D.7.1.r.4 - End Date
    end_date DATE,
    end_date_null_flavor VARCHAR(4) CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.7.1.r.5 - Comments
    comments TEXT,

    -- D.7.1.r.6 - Family History
    family_history BOOLEAN,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_med_history_sequence UNIQUE (patient_id, sequence_number)
);

CREATE INDEX idx_med_history_patient ON medical_history_episodes(patient_id);

-- ============================================================================
-- D.8.r: Past Drug History (Repeating)
-- ============================================================================

CREATE TABLE past_drug_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patient_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- D.8.r.1 - Drug Name
    drug_name VARCHAR(500),
    drug_name_null_flavor VARCHAR(4) CHECK (drug_name_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.8.r.2 - MPID (Medicinal Product ID)
    mpid VARCHAR(100),
    mpid_version VARCHAR(10),

    -- D.8.r.3 - PhPID (Pharmaceutical Product ID)
    phpid VARCHAR(100),
    phpid_version VARCHAR(10),

    -- D.8.r.4 - Start Date
    start_date DATE,
    start_date_null_flavor VARCHAR(4) CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.8.r.5 - End Date
    end_date DATE,
    end_date_null_flavor VARCHAR(4) CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.8.r.6a - Indication (MedDRA coded)
    indication_meddra_version VARCHAR(10),
    indication_meddra_code VARCHAR(20),

    -- D.8.r.7 - Reaction(s) (MedDRA coded)
    reaction_meddra_version VARCHAR(10),
    reaction_meddra_code VARCHAR(20),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_past_drug_sequence UNIQUE (patient_id, sequence_number)
);

CREATE INDEX idx_past_drug_patient ON past_drug_history(patient_id);

-- ============================================================================
-- D.9: Death Information
-- ============================================================================

CREATE TABLE patient_death_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patient_information(id) ON DELETE CASCADE,

    -- D.9.1 - Date of Death
    date_of_death DATE,
    date_of_death_null_flavor VARCHAR(4) CHECK (date_of_death_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.9.3 - Autopsy
    autopsy_performed BOOLEAN,

    -- D.9.4 - Autopsy Determined Cause of Death (handled in repeating table)

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_death_per_patient UNIQUE (patient_id)
);

-- D.9.2.r: Reported Cause(s) of Death (Repeating, MedDRA coded)
CREATE TABLE reported_causes_of_death (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    death_info_id UUID NOT NULL REFERENCES patient_death_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,
    meddra_version VARCHAR(10),
    meddra_code VARCHAR(20),
    comments TEXT,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_reported_death_cause UNIQUE (death_info_id, sequence_number)
);

-- D.9.4.r: Autopsy-determined Cause(s) of Death (Repeating, MedDRA coded)
CREATE TABLE autopsy_causes_of_death (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    death_info_id UUID NOT NULL REFERENCES patient_death_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,
    meddra_version VARCHAR(10),
    meddra_code VARCHAR(20),
    comments TEXT,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_autopsy_death_cause UNIQUE (death_info_id, sequence_number)
);

CREATE INDEX idx_death_info_patient ON patient_death_information(patient_id);

-- ============================================================================
-- D.10: Parent Information (for fetal/neonatal cases)
-- ============================================================================

CREATE TABLE parent_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL REFERENCES patient_information(id) ON DELETE CASCADE,

    -- D.10.1 - Parent Identification
    parent_identification VARCHAR(60),

    -- D.10.2.1 - Date of Birth of Parent
    parent_birth_date DATE,
    parent_birth_date_null_flavor VARCHAR(4) CHECK (parent_birth_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.10.2 - Parent Age
    parent_age DECIMAL(5,2),
    parent_age_null_flavor VARCHAR(4) CHECK (parent_age_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    parent_age_unit VARCHAR(3),  -- 800-804 (no hour unit for parents)

    -- D.10.3 - Last Menstrual Period Date
    last_menstrual_period_date DATE,
    last_menstrual_period_date_null_flavor VARCHAR(4) CHECK (last_menstrual_period_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.10.4 - Parent Weight (kg)
    weight_kg DECIMAL(6,2),

    -- D.10.5 - Parent Height (cm)
    height_cm DECIMAL(6,2),

    -- D.10.6 - Parent Sex
    sex VARCHAR(1) CHECK (sex IN ('0', '1', '2')),

    -- D.10.7.2 - Text for Relevant Parent Medical History
    medical_history_text TEXT,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_parent_per_patient UNIQUE (patient_id)
);

CREATE INDEX idx_parent_info_patient ON parent_information(patient_id);

-- ============================================================================
-- D.10.7.1.r: Parent Medical History Episodes (Repeating)
-- ============================================================================

CREATE TABLE parent_medical_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID NOT NULL REFERENCES parent_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- D.10.7.1.r.1a - MedDRA Version for Relevant Parent Medical History
    meddra_version VARCHAR(10),

    -- D.10.7.1.r.1b - Parent's Relevant Medical History (MedDRA code)
    meddra_code VARCHAR(20),

    -- D.10.7.1.r.2 - Start Date
    start_date DATE,
    start_date_null_flavor VARCHAR(4) CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.10.7.1.r.3 - Continuing
    continuing BOOLEAN,

    -- D.10.7.1.r.4 - End Date
    end_date DATE,
    end_date_null_flavor VARCHAR(4) CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.10.7.1.r.5 - Comments
    comments TEXT,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_parent_med_history_sequence UNIQUE (parent_id, sequence_number)
);

CREATE INDEX idx_parent_med_history_parent ON parent_medical_history(parent_id);

-- ============================================================================
-- D.10.8.r: Parent Past Drug History (Repeating)
-- ============================================================================

CREATE TABLE parent_past_drug_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_id UUID NOT NULL REFERENCES parent_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- D.10.8.r.1 - Drug Name
    drug_name VARCHAR(500),
    drug_name_null_flavor VARCHAR(4) CHECK (drug_name_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.10.8.r.2 - MPID (Medicinal Product ID)
    mpid VARCHAR(100),
    mpid_version VARCHAR(10),
    mfds_medicinal_product_version VARCHAR(20),
    mfds_medicinal_product_id VARCHAR(10),

    -- D.10.8.r.3 - PhPID (Pharmaceutical Product ID)
    phpid VARCHAR(100),
    phpid_version VARCHAR(10),

    -- D.10.8.r.4 - Start Date
    start_date DATE,
    start_date_null_flavor VARCHAR(4) CHECK (start_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.10.8.r.5 - End Date
    end_date DATE,
    end_date_null_flavor VARCHAR(4) CHECK (end_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- D.10.8.r.6a - MedDRA Version for Indication
    indication_meddra_version VARCHAR(10),

    -- D.10.8.r.6b - Indication (MedDRA code)
    indication_meddra_code VARCHAR(20),

    -- D.10.8.r.7a - MedDRA Version for Reaction
    reaction_meddra_version VARCHAR(10),

    -- D.10.8.r.7b - Reaction (MedDRA code)
    reaction_meddra_code VARCHAR(20),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_parent_past_drug_sequence UNIQUE (parent_id, sequence_number)
);

CREATE INDEX idx_parent_past_drug_parent ON parent_past_drug_history(parent_id);
