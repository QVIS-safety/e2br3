-- ============================================================================
-- SECTION G: Drug/Biological Information (Repeating - G.k)
-- ============================================================================

CREATE TABLE drug_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    source_product_presave_id UUID REFERENCES product_presaves(id) ON DELETE SET NULL,
    sequence_number INTEGER NOT NULL,  -- k value (drug index)

    -- G.k.1 - Characterization of Drug Role (MANDATORY - E2B(R3) codes)
    drug_characterization VARCHAR(1) NOT NULL CHECK (drug_characterization IN ('1', '2', '3', '4')),
    -- 1=Suspect, 2=Concomitant, 3=Interacting, 4=Drug Not Administered

    -- G.k.2.2 - Medicinal Product Name as Reported
    medicinal_product VARCHAR(500) NOT NULL,

    -- G.k.2.3.r - Substance/Specified Substance (repeating - handled in separate table)

    -- G.k.2.4.r - Identification of the Pharmaceutical Product (MPID)
    mpid VARCHAR(100),
    mpid_version VARCHAR(10),

    -- G.k.2.1.KR.1a/b - MFDS medicinal product fields
    mfds_mpid_version VARCHAR(20),
    mfds_mpid VARCHAR(10),

    -- G.k.2.5 - PhPID (Pharmaceutical Product Identifier)
    phpid VARCHAR(100),
    phpid_version VARCHAR(10),

    -- G.k.2.5 - Investigational Product Blinded
    investigational_product_blinded BOOLEAN,

    -- G.k.3.1 - Obtain Drug Country
    obtain_drug_country VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- G.k.3.2 - Proprietary/Brand Name
    brand_name VARCHAR(200),

    -- Application-level helper fields used by the UI
    drug_generic_name VARCHAR(500),
    drug_authorization_number VARCHAR(100),

    -- G.k.3.3.1 - Manufacturer Name
    manufacturer_name VARCHAR(100),

    -- G.k.3.3.2 - Manufacturer Country
    manufacturer_country VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- G.k.3.4 - Batch/Lot Number
    batch_lot_number VARCHAR(200),

    -- G.k.5 - Cumulative Dose to First Reaction
    cumulative_dose_first_reaction_value DECIMAL(15,5),
    cumulative_dose_first_reaction_unit VARCHAR(50),

    -- G.k.6 - Gestation Period at Time of Exposure
    gestation_period_exposure_value DECIMAL(10,2),
    gestation_period_exposure_unit VARCHAR(50),

    -- Application-level legacy dosage text
    dosage_text TEXT,

    -- G.k.7 - Action(s) Taken with Drug (E2B(R3) codes)
    action_taken VARCHAR(1) CHECK (action_taken IN ('1', '2', '3', '4', '5', '6')),
    -- 1=Withdrawn, 2=Dose reduced, 3=Dose increased, 4=Dose not changed,
    -- 5=Unknown, 6=Not applicable

    -- G.k.8 - Rechallenge/Recurrence Information
    rechallenge VARCHAR(1) CHECK (rechallenge IN ('1', '2', '3', '4')),
    -- 1=Yes, reaction recurred, 2=Yes, reaction did not recur, 3=No, 4=Unknown

    -- G.k.9 - Additional Information (handled in primary_sources table with drug FK)

    -- G.k.11 - Parent Dosage Information
    parent_dosage_text TEXT,

    -- FDA.G.k.10a - Additional Information on Drug (coded)
    fda_additional_info_coded VARCHAR(10),

    -- G.k.10.r - Additional Information on Drug (coded, repeating)
    drug_additional_info_codes_json JSONB,

    -- G.k.11 - Additional Information on Drug (free text)
    drug_additional_information TEXT,

    -- FDA.G.k.10.1 - FDA Specialized Product Category
    fda_specialized_product_category VARCHAR(60),

    -- FDA.G.k.12.r - Structured FDA device information payload
    fda_device_info_json JSONB,

    -- FDA.G.k.1.a - FDA Other Characterisation of Drug Role (1 = Similar Device)
    fda_other_characterization VARCHAR(10),

    deleted BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_drug_info_case ON drug_information(case_id);
CREATE INDEX idx_drug_info_source_presave ON drug_information(source_product_presave_id);
CREATE INDEX idx_drug_info_mpid ON drug_information(mpid);
CREATE INDEX idx_drug_info_mfds_mpid ON drug_information(mfds_mpid);
CREATE UNIQUE INDEX IF NOT EXISTS idx_drug_information_active_sequence_unique
    ON drug_information(case_id, sequence_number)
    WHERE deleted = false;

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

    -- G.k.2.3.r.1.KR.1a/b - MFDS substance fields
    mfds_version VARCHAR(20),
    mfds_id VARCHAR(10),

    -- G.k.2.3.r.3 - Strength (value + unit)
    strength_value DECIMAL(15,5),
    strength_unit VARCHAR(50),
    deleted BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_active_substances_drug ON drug_active_substances(drug_id);
CREATE UNIQUE INDEX idx_drug_active_substances_active_sequence_unique
    ON drug_active_substances(drug_id, sequence_number)
    WHERE deleted = false;

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
    continuing BOOLEAN,

    -- G.k.4.r.7 - Batch/Lot Number
    batch_lot_number VARCHAR(200),
    batch_lot_number_null_flavor VARCHAR(4) CHECK (batch_lot_number_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK', 'NA')),

    -- G.k.4.r.8 - Dosage Text
    dosage_text TEXT,

    -- G.k.4.r.9.1 - Pharmaceutical Dose Form
    dose_form VARCHAR(200),
    dose_form_termid VARCHAR(50),
    dose_form_termid_version VARCHAR(10),

    -- G.k.4.r.10 - Route of Administration
    route_of_administration VARCHAR(3),  -- E2B(R3) code list
    route_termid VARCHAR(50),
    route_termid_version VARCHAR(10),

    -- G.k.4.r.11 - Parent Route of Administration
    parent_route VARCHAR(50),
    parent_route_termid VARCHAR(50),
    parent_route_termid_version VARCHAR(10),

    -- Null Flavor Support (E2B(R3) compliant: NI, UNK, ASKU, NASK, MSK)
    first_administration_date_null_flavor VARCHAR(4) CHECK (first_administration_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    last_administration_date_null_flavor VARCHAR(4) CHECK (last_administration_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),
    deleted BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_dosage_info_drug ON dosage_information(drug_id);
CREATE UNIQUE INDEX idx_dosage_information_active_sequence_unique
    ON dosage_information(drug_id, sequence_number)
    WHERE deleted = false;

-- ============================================================================
-- G.k.6.r: Drug Indication(s) (Repeating, MedDRA coded)
-- ============================================================================

CREATE TABLE drug_indications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- G.k.6.r.1 - Indication (free text)
    indication_text VARCHAR(500),
    indication_text_null_flavor VARCHAR(4) CHECK (indication_text_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK', 'NA')),

    -- G.k.6.r.2 - Indication (MedDRA coded)
    indication_meddra_version VARCHAR(10),
    indication_meddra_code VARCHAR(20),
    deleted BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_drug_indications_drug ON drug_indications(drug_id);
CREATE UNIQUE INDEX idx_drug_indications_active_sequence_unique
    ON drug_indications(drug_id, sequence_number)
    WHERE deleted = false;

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
    deleted BOOLEAN NOT NULL DEFAULT false,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_drug_device_characteristics_drug ON drug_device_characteristics(drug_id);
CREATE UNIQUE INDEX idx_drug_device_characteristics_active_sequence_unique
    ON drug_device_characteristics(drug_id, sequence_number)
    WHERE deleted = false;

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
    deleted BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_drug_recurrence_drug ON drug_recurrence_information(drug_id);
CREATE UNIQUE INDEX idx_drug_recurrence_information_active_sequence_unique
    ON drug_recurrence_information(drug_id, sequence_number)
    WHERE deleted = false;

-- ============================================================================
-- G.k.9.i: Drug-Reaction Assessment (Causality)
-- Links each drug (G.k) to each reaction (E.i) with causality assessment data
-- ============================================================================

CREATE TABLE drug_reaction_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drug_id UUID NOT NULL REFERENCES drug_information(id) ON DELETE CASCADE,
    reaction_id UUID NOT NULL REFERENCES reactions(id) ON DELETE CASCADE,

    -- G.k.9.i.3.1a/b - Time Interval between Beginning of Drug Administration and Start of Reaction / Event
    administration_start_interval_value DECIMAL(10,2),
    administration_start_interval_unit VARCHAR(3),  -- 800-805 (decade, year, month, week, day, hour)

    -- G.k.9.i.3.2a/b - Time Interval between Last Dose of Drug and Start of Reaction / Event
    last_dose_interval_value DECIMAL(10,2),
    last_dose_interval_unit VARCHAR(3),  -- 800-805 (decade, year, month, week, day, hour)

    -- G.k.9.i.4.r.1 - Did Reaction Recur on Readministration - Action
    recurrence_action VARCHAR(1) CHECK (recurrence_action IN ('1', '2', '3', '4')),
    -- 1=Drug readministered, 2=Drug not readministered, 3=Unknown, 4=Not applicable

    -- G.k.9.i.4.r.2a - MedDRA Version for Reported Term for Reaction Recurred
    recurrence_meddra_version VARCHAR(10),

    -- G.k.9.i.4.r.2b - Reported Term for Reaction Recurred (MedDRA code)
    recurrence_meddra_code VARCHAR(20),

    -- G.k.9.i.4.r.3 - Did Reaction Recur on Readministration
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
    deleted BOOLEAN NOT NULL DEFAULT false,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_relatedness_assessments_parent ON relatedness_assessments(drug_reaction_assessment_id);
CREATE UNIQUE INDEX idx_relatedness_assessments_active_sequence_unique
    ON relatedness_assessments(drug_reaction_assessment_id, sequence_number)
    WHERE deleted = false;
