-- ============================================================================
-- SECTION G: Drug/Biological Information (Repeating - G.k)
-- ============================================================================

CREATE TABLE drug_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,  -- k value (drug index)

    -- G.k.1 - Characterization of Drug Role (MANDATORY - E2B(R3) codes)
    drug_characterization VARCHAR(1) NOT NULL CHECK (drug_characterization IN ('1', '2', '3')),
    -- 1=Suspect, 2=Concomitant, 3=Interacting

    -- G.k.2.2 - Medicinal Product Name as Reported
    medicinal_product VARCHAR(500) NOT NULL,

    -- G.k.2.3.r - Substance/Specified Substance (repeating - handled in separate table)

    -- G.k.2.4.r - Identification of the Pharmaceutical Product (MPID)
    mpid VARCHAR(100),
    mpid_version VARCHAR(10),

    -- G.k.2.5 - PhPID (Pharmaceutical Product Identifier)
    phpid VARCHAR(100),
    phpid_version VARCHAR(10),

    -- G.k.2.5 - Investigational Product Blinded
    investigational_product_blinded BOOLEAN,

    -- G.k.3.1 - Obtain Drug Country
    obtain_drug_country VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- G.k.3.2 - Proprietary/Brand Name
    brand_name VARCHAR(200),

    -- G.k.3.3.1 - Manufacturer Name
    manufacturer_name VARCHAR(100),

    -- G.k.3.3.2 - Manufacturer Country
    manufacturer_country VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- G.k.3.4 - Batch/Lot Number
    batch_lot_number VARCHAR(200),

    -- G.k.5 - Dosage Text
    dosage_text TEXT,

    -- G.k.7 - Action(s) Taken with Drug (E2B(R3) codes)
    action_taken VARCHAR(1) CHECK (action_taken IN ('1', '2', '3', '4', '5', '6')),
    -- 1=Withdrawn, 2=Dose reduced, 3=Dose increased, 4=Dose not changed,
    -- 5=Unknown, 6=Not applicable

    -- G.k.8 - Rechallenge/Recurrence Information
    rechallenge VARCHAR(1) CHECK (rechallenge IN ('1', '2', '3', '4')),
    -- 1=Yes, reaction recurred, 2=Yes, reaction did not recur, 3=No, 4=Unknown

    -- G.k.9 - Additional Information (handled in primary_sources table with drug FK)

    -- G.k.10 - Parent Route of Administration (free text)
    parent_route VARCHAR(50),
    parent_route_termid VARCHAR(50),
    parent_route_termid_version VARCHAR(10),

    -- G.k.11 - Parent Dosage Information
    parent_dosage_text TEXT,

    -- FDA.G.k.10a - Additional Information on Drug (coded)
    fda_additional_info_coded VARCHAR(10),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_drug_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_drug_info_case ON drug_information(case_id);
CREATE INDEX idx_drug_info_mpid ON drug_information(mpid);

-- ============================================================================
-- G.k.2.3.r: Active Substance(s) (Repeating)
-- ============================================================================

CREATE TABLE drug_active_substances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- G.k.2.3.r.1 - Substance Name
    substance_name VARCHAR(500),

    -- G.k.2.3.r.2 - Substance TermID (SUB TermID)
    substance_termid VARCHAR(100),
    substance_termid_version VARCHAR(10),

    -- G.k.2.3.r.3 - Strength (value + unit)
    strength_value DECIMAL(15,5),
    strength_unit VARCHAR(50),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_substance_sequence UNIQUE (drug_id, sequence_number)
);

CREATE INDEX idx_active_substances_drug ON drug_active_substances(drug_id);

-- ============================================================================
-- G.k.4.r: Dosage and Relevant Information (Repeating)
-- ============================================================================

CREATE TABLE dosage_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,  -- r value

    -- G.k.4.r.1 - Dose (value + unit)
    dose_value DECIMAL(15,5),
    dose_unit VARCHAR(50),

    -- G.k.4.r.2 - Number of Separate Dosages
    number_of_units INTEGER,

    -- G.k.4.r.3 - Dose Frequency (value + unit)
    frequency_value DECIMAL(10,2),
    frequency_unit VARCHAR(50),

    -- G.k.4.r.4 - Date/Time of First Administration
    first_administration_date DATE,
    first_administration_time TIME,

    -- G.k.4.r.5 - Date/Time of Last Administration
    last_administration_date DATE,
    last_administration_time TIME,

    -- G.k.4.r.6 - Duration of Drug Administration
    duration_value DECIMAL(10,2),
    duration_unit VARCHAR(3),  -- 800-805 codes

    -- G.k.4.r.7 - Batch/Lot Number
    batch_lot_number VARCHAR(200),

    -- G.k.4.r.8 - Dosage Text
    dosage_text TEXT,

    -- G.k.4.r.9.1 - Pharmaceutical Dose Form
    dose_form VARCHAR(200),
    dose_form_termid VARCHAR(50),
    dose_form_termid_version VARCHAR(10),

    -- G.k.4.r.10 - Route of Administration
    route_of_administration VARCHAR(3),  -- E2B(R3) code list

    -- G.k.4.r.11 - Parent Route of Administration
    parent_route VARCHAR(50),
    parent_route_termid VARCHAR(50),
    parent_route_termid_version VARCHAR(10),

    -- Null Flavor Support (E2B(R3) compliant: NI, UNK, ASKU, NASK, MSK)
    first_administration_date_null_flavor VARCHAR(4) CHECK (first_administration_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    last_administration_date_null_flavor VARCHAR(4) CHECK (last_administration_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_dosage_sequence UNIQUE (drug_id, sequence_number)
);

CREATE INDEX idx_dosage_info_drug ON dosage_information(drug_id);

-- ============================================================================
-- G.k.6.r: Drug Indication(s) (Repeating, MedDRA coded)
-- ============================================================================

CREATE TABLE drug_indications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- G.k.6.r.1 - Indication (free text)
    indication_text VARCHAR(500),

    -- G.k.6.r.2 - Indication (MedDRA coded)
    indication_meddra_version VARCHAR(10),
    indication_meddra_code VARCHAR(20),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_indication_sequence UNIQUE (drug_id, sequence_number)
);

CREATE INDEX idx_drug_indications_drug ON drug_indications(drug_id);

-- ============================================================================
-- FDA Scenario 7: Device/Product Characteristics (Repeating)
-- ============================================================================

CREATE TABLE drug_device_characteristics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    code VARCHAR(50),
    code_system VARCHAR(200),
    code_display_name VARCHAR(200),
    value_type VARCHAR(10),
    value_value VARCHAR(200),
    value_code VARCHAR(50),
    value_code_system VARCHAR(200),
    value_display_name VARCHAR(200),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_drug_device_characteristic_sequence UNIQUE (drug_id, sequence_number)
);

CREATE INDEX idx_drug_device_characteristics_drug ON drug_device_characteristics(drug_id);

-- ============================================================================
-- G.k.8.r: Drug Recurrence Information (Repeating)
-- Structured recurrence data for rechallenge scenarios
-- ============================================================================

CREATE TABLE drug_recurrence_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- G.k.8.r.1 - Rechallenge Action
    rechallenge_action VARCHAR(1) CHECK (rechallenge_action IN ('1', '2', '3', '4')),
    -- 1=Drug readministered, 2=Drug not readministered, 3=Unknown, 4=Not applicable

    -- G.k.8.r.2a - MedDRA Version
    reaction_meddra_version VARCHAR(10),

    -- G.k.8.r.2b - Reaction Recurred (MedDRA code)
    reaction_meddra_code VARCHAR(20),

    -- G.k.8.r.3 - Did Reaction Recur on Readministration
    reaction_recurred VARCHAR(1) CHECK (reaction_recurred IN ('1', '2', '3')),
    -- 1=Yes, 2=No, 3=Unknown

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_drug_recurrence_sequence UNIQUE (drug_id, sequence_number)
);

CREATE INDEX idx_drug_recurrence_drug ON drug_recurrence_information(drug_id);

-- ============================================================================
-- G.k.9.i: Drug-Reaction Assessment (Causality)
-- Links each drug (G.k) to each reaction (E.i) with causality assessment data
-- ============================================================================

CREATE TABLE drug_reaction_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    reaction_id UUID NOT NULL REFERENCES reactions(id) ON DELETE CASCADE,

    -- G.k.9.i.1 - Time Interval between Drug Administration and Reaction Onset
    time_interval_value DECIMAL(10,2),
    time_interval_unit VARCHAR(3),  -- 800-805 (decade, year, month, week, day, hour)

    -- G.k.9.i.3.1 - Did Reaction Recur on Readministration - Action
    recurrence_action VARCHAR(1) CHECK (recurrence_action IN ('1', '2', '3', '4')),
    -- 1=Drug readministered, 2=Drug not readministered, 3=Unknown, 4=Not applicable

    -- G.k.9.i.3.2a - MedDRA Version for Reported Term for Reaction Recurred
    recurrence_meddra_version VARCHAR(10),

    -- G.k.9.i.3.2b - Reported Term for Reaction Recurred (MedDRA code)
    recurrence_meddra_code VARCHAR(20),

    -- G.k.9.i.4 - Did Reaction Recur on Readministration
    reaction_recurred VARCHAR(1) CHECK (reaction_recurred IN ('1', '2', '3')),
    -- 1=Yes, 2=No, 3=Unknown

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    -- Each drug-reaction pair should have only one assessment record
    CONSTRAINT unique_drug_reaction_assessment UNIQUE (drug_id, reaction_id)
);

CREATE INDEX idx_drug_reaction_assessments_drug ON drug_reaction_assessments(drug_id);
CREATE INDEX idx_drug_reaction_assessments_reaction ON drug_reaction_assessments(reaction_id);

-- ============================================================================
-- G.k.9.i.2.r: Relatedness Assessments (Repeating)
-- Multiple assessments per drug-reaction pair from different sources
-- ============================================================================

CREATE TABLE relatedness_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_reaction_assessment_id UUID NOT NULL REFERENCES drug_reaction_assessments(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- G.k.9.i.2.r.1 - Source of Assessment
    source_of_assessment VARCHAR(100),

    -- G.k.9.i.2.r.2 - Method of Assessment
    method_of_assessment VARCHAR(100),

    -- G.k.9.i.2.r.3 - Result of Assessment
    result_of_assessment VARCHAR(50),
    -- MFDS.G.k.9.i.2.r.3.KR.2 - Additional KR assessment result text
    result_of_assessment_kr2 VARCHAR(2000),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_relatedness_sequence UNIQUE (drug_reaction_assessment_id, sequence_number)
);

CREATE INDEX idx_relatedness_assessments_parent ON relatedness_assessments(drug_reaction_assessment_id);
