-- ============================================================================
-- Controlled Terminologies
-- ============================================================================

-- MedDRA Terms (Medical Dictionary for Regulatory Activities)
CREATE TABLE meddra_terms (
    id BIGSERIAL PRIMARY KEY,
    audit_id UUID NOT NULL DEFAULT gen_random_uuid(),
    code VARCHAR(20) NOT NULL,
    term VARCHAR(500) NOT NULL,
    level VARCHAR(10) NOT NULL,  -- LLT, PT, HLT, HLGT, SOC
    version VARCHAR(10) NOT NULL,
    language VARCHAR(2) DEFAULT 'en',  -- ISO 639-1
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_meddra_code_version UNIQUE (code, version, language),
    CONSTRAINT unique_meddra_audit_id UNIQUE (audit_id)
);

CREATE INDEX idx_meddra_code ON meddra_terms(code);
CREATE INDEX idx_meddra_term ON meddra_terms USING gin(to_tsvector('english', term));
CREATE INDEX idx_meddra_version ON meddra_terms(version);
CREATE INDEX idx_meddra_level ON meddra_terms(level);

-- WHODrug Products (WHO Drug Dictionary)
CREATE TABLE whodrug_products (
    id BIGSERIAL PRIMARY KEY,
    audit_id UUID NOT NULL DEFAULT gen_random_uuid(),
    code VARCHAR(20) NOT NULL,
    drug_name TEXT NOT NULL,
    atc_code VARCHAR(20),  -- Anatomical Therapeutic Chemical code
    version VARCHAR(10) NOT NULL,
    language VARCHAR(2) DEFAULT 'en',
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_whodrug_code_version UNIQUE (code, version, language),
    CONSTRAINT unique_whodrug_audit_id UNIQUE (audit_id)
);

CREATE INDEX idx_whodrug_code ON whodrug_products(code);
CREATE INDEX idx_whodrug_name ON whodrug_products USING gin(to_tsvector('english', drug_name));
CREATE INDEX idx_whodrug_atc ON whodrug_products(atc_code);

-- Terminology release workflow (staged/approved/active)
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

-- ISO Country Codes
CREATE TABLE iso_countries (
    code VARCHAR(2) PRIMARY KEY,  -- ISO 3166-1 alpha-2
    audit_id UUID NOT NULL DEFAULT gen_random_uuid(),
    name VARCHAR(200) NOT NULL,
    active BOOLEAN DEFAULT true,

    CONSTRAINT unique_iso_countries_audit_id UNIQUE (audit_id)
);

-- E2B(R3) Code Lists (enumerated values)
CREATE TABLE e2b_code_lists (
    id SERIAL PRIMARY KEY,
    audit_id UUID NOT NULL DEFAULT gen_random_uuid(),
    list_name VARCHAR(100) NOT NULL,  -- e.g., 'report_type', 'drug_action'
    code VARCHAR(10) NOT NULL,
    display_name VARCHAR(200) NOT NULL,
    description TEXT,
    sort_order INTEGER,
    active BOOLEAN DEFAULT true,

    CONSTRAINT unique_code_per_list UNIQUE (list_name, code),
    CONSTRAINT unique_e2b_code_lists_audit_id UNIQUE (audit_id)
);

CREATE INDEX idx_code_lists_name ON e2b_code_lists(list_name);

-- Pre-populate common E2B(R3) code lists
INSERT INTO e2b_code_lists (list_name, code, display_name, sort_order) VALUES
-- Report Type (C.1.3)
('report_type', '1', 'Spontaneous report', 1),
('report_type', '2', 'Report from study', 2),
('report_type', '3', 'Other', 3),
('report_type', '4', 'Not available to sender', 4),

-- Sender Type (C.3.1)
('sender_type', '1', 'Pharmaceutical company', 1),
('sender_type', '2', 'Regulatory authority', 2),
('sender_type', '3', 'Health professional', 3),
('sender_type', '4', 'Regional pharmacovigilance center', 4),
('sender_type', '5', 'WHO collaborating center for international drug monitoring', 5),
('sender_type', '6', 'Other', 6),

-- Qualification (C.2.r.4)
('qualification', '1', 'Physician', 1),
('qualification', '2', 'Pharmacist', 2),
('qualification', '3', 'Other health professional', 3),
('qualification', '4', 'Lawyer', 4),
('qualification', '5', 'Consumer or other non health professional', 5),

-- Sex (D.5)
('sex', '0', 'Unknown', 0),
('sex', '1', 'Male', 1),
('sex', '2', 'Female', 2),

-- Age Group (D.2.3)
('age_group', '1', 'Neonate', 1),
('age_group', '2', 'Infant', 2),
('age_group', '3', 'Child', 3),
('age_group', '4', 'Adolescent', 4),
('age_group', '5', 'Adult', 5),
('age_group', '6', 'Elderly', 6),

-- Age Unit (D.2.2)
('age_unit', '800', 'Decade', 1),
('age_unit', '801', 'Year', 2),
('age_unit', '802', 'Month', 3),
('age_unit', '803', 'Week', 4),
('age_unit', '804', 'Day', 5),
('age_unit', '805', 'Hour', 6),

-- Reaction Outcome (E.i.7)
('reaction_outcome', '0', 'Unknown', 0),
('reaction_outcome', '1', 'Recovered/resolved', 1),
('reaction_outcome', '2', 'Recovering/resolving', 2),
('reaction_outcome', '3', 'Not recovered/not resolved', 3),
('reaction_outcome', '4', 'Recovered/resolved with sequelae', 4),
('reaction_outcome', '5', 'Fatal', 5),

-- Drug Characterization (G.k.1)
('drug_characterization', '1', 'Suspect', 1),
('drug_characterization', '2', 'Concomitant', 2),
('drug_characterization', '3', 'Interacting', 3),

-- Drug Action Taken (G.k.7)
('drug_action', '1', 'Drug withdrawn', 1),
('drug_action', '2', 'Dose reduced', 2),
('drug_action', '3', 'Dose increased', 3),
('drug_action', '4', 'Dose not changed', 4),
('drug_action', '5', 'Unknown', 5),
('drug_action', '6', 'Not applicable', 6),

-- Rechallenge Result (G.k.8)
('rechallenge', '1', 'Yes, reaction recurred', 1),
('rechallenge', '2', 'Yes, reaction did not recur', 2),
('rechallenge', '3', 'No', 3),
('rechallenge', '4', 'Unknown', 4),

-- Primary Source Regulatory (C.2.r.5)
('primary_source_regulatory', '1', 'Yes', 1),
('primary_source_regulatory', '2', 'No', 2),

-- Study Type (C.5.4)
('study_type', '1', 'Clinical trial', 1),
('study_type', '2', 'Individual patient use', 2),
('study_type', '3', 'Other studies', 3),

-- Term Highlighted (E.i.3.1)
('term_highlighted', '1', 'Yes, highlighted by reporter, NOT serious', 1),
('term_highlighted', '2', 'No, not highlighted by reporter, NOT serious', 2),
('term_highlighted', '3', 'Yes, highlighted by reporter, SERIOUS', 3),
('term_highlighted', '4', 'No, not highlighted by reporter, SERIOUS', 4),

-- Medical Confirmation (E.i.8)
('medical_confirmation', '1', 'Yes', 1),
('medical_confirmation', '2', 'No', 2),
('medical_confirmation', '3', 'Unknown', 3),

-- Receiver Type (A.1.4) - same codes as sender_type
('receiver_type', '1', 'Pharmaceutical company', 1),
('receiver_type', '2', 'Regulatory authority', 2),
('receiver_type', '3', 'Health professional', 3),
('receiver_type', '4', 'Regional pharmacovigilance center', 4),
('receiver_type', '5', 'WHO collaborating center for international drug monitoring', 5),
('receiver_type', '6', 'Other', 6),

-- Recurrence Action (G.k.9.i.3.1)
('recurrence_action', '1', 'Drug readministered', 1),
('recurrence_action', '2', 'Drug not readministered', 2),
('recurrence_action', '3', 'Unknown', 3),
('recurrence_action', '4', 'Not applicable', 4),

-- Reaction Recurred (G.k.9.i.4)
('reaction_recurred', '1', 'Yes', 1),
('reaction_recurred', '2', 'No', 2),
('reaction_recurred', '3', 'Unknown', 3),

-- Null Flavor Codes (E2B(R3) compliant)
('null_flavor', 'NI', 'No Information', 1),
('null_flavor', 'UNK', 'Unknown', 2),
('null_flavor', 'ASKU', 'Asked but Unknown', 3),
('null_flavor', 'NASK', 'Not Asked', 4),
('null_flavor', 'MSK', 'Masked', 5),

-- Route of Administration (G.k.4.r.10) - Common routes
('route_of_administration', '001', 'Auricular (otic)', 1),
('route_of_administration', '002', 'Buccal', 2),
('route_of_administration', '003', 'Cutaneous', 3),
('route_of_administration', '004', 'Dental', 4),
('route_of_administration', '005', 'Endocervical', 5),
('route_of_administration', '006', 'Endosinusial', 6),
('route_of_administration', '007', 'Endotracheal', 7),
('route_of_administration', '008', 'Epidural', 8),
('route_of_administration', '009', 'Extra-amniotic', 9),
('route_of_administration', '010', 'Hemodialysis', 10),
('route_of_administration', '011', 'Intra-corporus cavernosum', 11),
('route_of_administration', '012', 'Intra-amniotic', 12),
('route_of_administration', '013', 'Intra-arterial', 13),
('route_of_administration', '014', 'Intra-articular', 14),
('route_of_administration', '015', 'Intra-uterine', 15),
('route_of_administration', '016', 'Intracardiac', 16),
('route_of_administration', '017', 'Intracavernous', 17),
('route_of_administration', '018', 'Intracerebral', 18),
('route_of_administration', '019', 'Intracervical', 19),
('route_of_administration', '020', 'Intracisternal', 20),
('route_of_administration', '021', 'Intracorneal', 21),
('route_of_administration', '022', 'Intracoronary', 22),
('route_of_administration', '023', 'Intradermal', 23),
('route_of_administration', '024', 'Intradiscal', 24),
('route_of_administration', '025', 'Intrahepatic', 25),
('route_of_administration', '026', 'Intralesional', 26),
('route_of_administration', '027', 'Intralymphatic', 27),
('route_of_administration', '028', 'Intramedullar (bone marrow)', 28),
('route_of_administration', '029', 'Intrameningeal', 29),
('route_of_administration', '030', 'Intramuscular', 30),
('route_of_administration', '031', 'Intraocular', 31),
('route_of_administration', '032', 'Intrapericardial', 32),
('route_of_administration', '033', 'Intraperitoneal', 33),
('route_of_administration', '034', 'Intrapleural', 34),
('route_of_administration', '035', 'Intrasynovial', 35),
('route_of_administration', '036', 'Intrathecal', 36),
('route_of_administration', '037', 'Intrathoracic', 37),
('route_of_administration', '038', 'Intratracheal', 38),
('route_of_administration', '039', 'Intratumour', 39),
('route_of_administration', '040', 'Intratympanic', 40),
('route_of_administration', '041', 'Intravenous (not elsewhere classified)', 41),
('route_of_administration', '042', 'Intravenous bolus', 42),
('route_of_administration', '043', 'Intravenous drip', 43),
('route_of_administration', '044', 'Intravesical', 44),
('route_of_administration', '045', 'Iontophoresis', 45),
('route_of_administration', '046', 'Nasal', 46),
('route_of_administration', '047', 'Ocular', 47),
('route_of_administration', '048', 'Oral', 48),
('route_of_administration', '049', 'Oropharyngeal', 49),
('route_of_administration', '050', 'Other', 50),
('route_of_administration', '051', 'Parenteral', 51),
('route_of_administration', '052', 'Periarticular', 52),
('route_of_administration', '053', 'Perineural', 53),
('route_of_administration', '054', 'Rectal', 54),
('route_of_administration', '055', 'Respiratory (inhalation)', 55),
('route_of_administration', '056', 'Retrobulbar', 56),
('route_of_administration', '057', 'Sunconjunctival', 57),
('route_of_administration', '058', 'Subcutaneous', 58),
('route_of_administration', '059', 'Subdermal', 59),
('route_of_administration', '060', 'Sublingual', 60),
('route_of_administration', '061', 'Topical', 61),
('route_of_administration', '062', 'Transdermal', 62),
('route_of_administration', '063', 'Transmammary', 63),
('route_of_administration', '064', 'Transplacental', 64),
('route_of_administration', '065', 'Unknown', 65),
('route_of_administration', '066', 'Urethral', 66),
('route_of_administration', '067', 'Vaginal', 67);

-- ============================================================================
-- UCUM Units Reference Table
-- Measurement units for E2B(R3) compliance
-- ============================================================================

CREATE TABLE ucum_units (
    id SERIAL PRIMARY KEY,
    code VARCHAR(20) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    description TEXT,
    unit_type VARCHAR(50),  -- time, mass, volume, etc.
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_ucum_code UNIQUE (code)
);

-- Common UCUM units for E2B(R3)
INSERT INTO ucum_units (code, display_name, unit_type) VALUES
-- Time units
('s', 'second', 'time'),
('min', 'minute', 'time'),
('h', 'hour', 'time'),
('d', 'day', 'time'),
('wk', 'week', 'time'),
('mo', 'month', 'time'),
('a', 'year', 'time'),
-- Mass units
('kg', 'kilogram', 'mass'),
('g', 'gram', 'mass'),
('mg', 'milligram', 'mass'),
('ug', 'microgram', 'mass'),
('ng', 'nanogram', 'mass'),
-- Volume units
('L', 'liter', 'volume'),
('mL', 'milliliter', 'volume'),
('uL', 'microliter', 'volume'),
-- Length units
('m', 'meter', 'length'),
('cm', 'centimeter', 'length'),
('mm', 'millimeter', 'length'),
-- Concentration units
('mg/mL', 'milligram per milliliter', 'concentration'),
('mg/dL', 'milligram per deciliter', 'concentration'),
('mg/L', 'milligram per liter', 'concentration'),
('ug/mL', 'microgram per milliliter', 'concentration'),
('g/L', 'gram per liter', 'concentration'),
('mmol/L', 'millimole per liter', 'concentration'),
('U/L', 'unit per liter', 'activity concentration'),
-- Dose units
('mg/kg', 'milligram per kilogram', 'dose'),
('mg/m2', 'milligram per square meter', 'dose'),
('[IU]', 'international unit', 'dose'),
('[IU]/kg', 'international unit per kilogram', 'dose'),
-- Other common units
('%', 'percent', 'ratio'),
('1', 'unity (count)', 'count');
