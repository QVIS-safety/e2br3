-- ============================================================================
-- SECTION C: Safety Report Identification
-- ============================================================================

CREATE TABLE safety_report_identification (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,

    -- C.1.2 - Date of Creation (MANDATORY)
    transmission_date DATE,
    transmission_date_null_flavor VARCHAR(4) CHECK (transmission_date_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- C.1.3 - Type of Report (MANDATORY - E2B(R3) codes)
    report_type VARCHAR(1) NOT NULL CHECK (report_type IN ('1', '2', '3', '4')),
    -- 1=Spontaneous report, 2=Report from study, 3=Other, 4=Not available

    -- C.1.4 - Date Report Was First Received from Source (MANDATORY)
    date_first_received_from_source DATE,
    date_first_received_from_source_null_flavor VARCHAR(4) CHECK (date_first_received_from_source_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- C.1.5 - Date of Most Recent Information (MANDATORY)
    date_of_most_recent_information DATE,
    date_of_most_recent_information_null_flavor VARCHAR(4) CHECK (date_of_most_recent_information_null_flavor IN ('NI', 'UNK', 'ASKU', 'NASK', 'MSK')),

    -- C.1.7 - Fulfils Expedited Criteria (MANDATORY)
    fulfil_expedited_criteria BOOLEAN NOT NULL,

    -- FDA.C.1.7.1 - Local Criteria Report Type (FDA)
    local_criteria_report_type VARCHAR(10),

    -- FDA.C.1.12 - Combination Product Report Indicator (FDA)
    combination_product_report_indicator VARCHAR(10),

    -- C.1.8.1 - Worldwide Unique Case Identification
    worldwide_unique_id VARCHAR(100),

    -- C.1.10.r - Linked Report Numbers (handled in separate table)

    -- C.1.11.1 - Nullification/Amendment Code
    nullification_code VARCHAR(10),

    -- C.1.11.2 - Nullification Reason
    nullification_reason TEXT,

    -- Receiver Organization
    receiver_organization VARCHAR(200),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_identification_per_case UNIQUE (case_id)
);

CREATE INDEX idx_safety_report_id_case ON safety_report_identification(case_id);
CREATE INDEX idx_safety_report_id_worldwide ON safety_report_identification(worldwide_unique_id);

-- ============================================================================
-- SECTION C.3: Sender Information (MANDATORY fields)
-- ============================================================================

CREATE TABLE sender_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,

    -- C.3.1 - Sender Type (MANDATORY - E2B(R3) codes)
    sender_type VARCHAR(1) NOT NULL CHECK (sender_type IN ('1', '2', '3', '4', '5', '6')),
    -- 1=Pharmaceutical company, 2=Regulatory authority, 3=Health professional
    -- 4=Regional pharmacovigilance center, 5=WHO collaborating center, 6=Other

    -- C.3.2 - Sender's Organisation (MANDATORY)
    organization_name VARCHAR(200) NOT NULL,
    department VARCHAR(100),
    street_address VARCHAR(100),
    city VARCHAR(50),
    state VARCHAR(40),
    postcode VARCHAR(15),
    country_code VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- C.3.3 - Person Responsible for Sending
    person_title VARCHAR(50),
    person_given_name VARCHAR(60),
    person_middle_name VARCHAR(60),
    person_family_name VARCHAR(60),

    -- C.3.4 - Contact Information
    telephone VARCHAR(33),
    fax VARCHAR(33),
    email VARCHAR(100),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_sender_per_case UNIQUE (case_id)
);

CREATE INDEX idx_sender_info_case ON sender_information(case_id);

-- ============================================================================
-- SECTION C.4.r: Literature References (Repeating)
-- ============================================================================

CREATE TABLE literature_references (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    reference_text TEXT NOT NULL,  -- C.4.r
    sequence_number INTEGER NOT NULL,  -- For ordering
    document_base64 TEXT, -- C.4.r.2 Included Documents (base64)
    media_type VARCHAR(100),
    representation VARCHAR(10),
    compression VARCHAR(10),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_lit_ref_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_literature_refs_case ON literature_references(case_id);

-- ============================================================================
-- SECTION C.1.6.1.r: Documents Held by Sender (Repeating)
-- ============================================================================

CREATE TABLE documents_held_by_sender (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    title TEXT, -- C.1.6.1.r.1
    document_base64 TEXT, -- C.1.6.1.r.2 Included Documents
    media_type VARCHAR(100),
    representation VARCHAR(10),
    compression VARCHAR(10),
    sequence_number INTEGER NOT NULL,

    -- Audit fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_documents_held_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_documents_held_by_sender_case ON documents_held_by_sender(case_id);

-- ============================================================================
-- SECTION C.5: Study Information
-- ============================================================================

CREATE TABLE study_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,

    -- C.5.1 - Study Name
    study_name VARCHAR(500),

    -- C.5.2 - Sponsor Study Number
    sponsor_study_number VARCHAR(100),

    -- C.5.3 - Study Type Reaction
    study_type_reaction VARCHAR(2),  -- E2B(R3) code list
    -- MFDS.C.5.4.KR.1 - Other studies type (KR extension)
    study_type_reaction_kr1 VARCHAR(1) CHECK (study_type_reaction_kr1 IN ('1', '2', '3', '4')),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_study_per_case UNIQUE (case_id)
);

-- C.5.1.r - Study Registration Numbers (Repeating)
CREATE TABLE study_registration_numbers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    study_information_id UUID NOT NULL REFERENCES study_information(id) ON DELETE CASCADE,
    registration_number VARCHAR(100) NOT NULL,
    country_code VARCHAR(2),  -- ISO 3166-1 alpha-2
    sequence_number INTEGER NOT NULL,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_study_reg_num UNIQUE (study_information_id, sequence_number)
);

CREATE INDEX idx_study_info_case ON study_information(case_id);
CREATE INDEX idx_study_reg_nums ON study_registration_numbers(study_information_id);

-- ============================================================================
-- SECTION C.2.r: Primary Source Information (Repeating)
-- ============================================================================

CREATE TABLE primary_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,  -- For ordering multiple reporters

    -- C.2.r.1 - Reporter's Name
    reporter_title VARCHAR(50),
    reporter_given_name VARCHAR(60),
    reporter_middle_name VARCHAR(60),
    reporter_family_name VARCHAR(60),

    -- C.2.r.2 - Reporter's Address and Contact
    organization VARCHAR(60),
    department VARCHAR(60),
    street VARCHAR(100),
    city VARCHAR(35),
    state VARCHAR(40),
    postcode VARCHAR(15),
    telephone VARCHAR(33),

    -- C.2.r.3 - Country Code
    country_code VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- Email (not in spec but commonly used)
    email VARCHAR(100),

    -- C.2.r.4 - Qualification (MANDATORY within primary source - E2B(R3) codes)
    qualification VARCHAR(1) CHECK (qualification IN ('1', '2', '3', '4', '5')),
    -- 1=Physician, 2=Pharmacist, 3=Other health professional, 4=Lawyer, 5=Consumer
    -- MFDS.C.2.r.4.KR.1 - Other health professional type (KR extension)
    qualification_kr1 VARCHAR(1) CHECK (qualification_kr1 IN ('1', '2')),

    -- C.2.r.5 - Primary Source for Regulatory Purposes (MANDATORY)
    primary_source_regulatory VARCHAR(1) CHECK (primary_source_regulatory IN ('1', '2')),
    -- 1=Yes, 2=No

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_primary_source_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_primary_sources_case ON primary_sources(case_id);

-- ============================================================================
-- SECTION A: Receiver Information
-- A.1.4 through A.1.5.10 - Receiver details for routing to regulatory authorities
-- ============================================================================

CREATE TABLE receiver_information (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,

    -- A.1.4 - Receiver Type
    receiver_type VARCHAR(1) CHECK (receiver_type IN ('1', '2', '3', '4', '5', '6')),
    -- 1=Pharmaceutical company, 2=Regulatory authority, 3=Health professional
    -- 4=Regional pharmacovigilance center, 5=WHO collaborating center, 6=Other

    -- A.1.5.1 - Receiver Organization
    organization_name VARCHAR(100),

    -- A.1.5.2 - Receiver Department
    department VARCHAR(60),

    -- A.1.5.3 - Receiver Street Address
    street_address VARCHAR(100),

    -- A.1.5.4 - Receiver City
    city VARCHAR(35),

    -- A.1.5.5 - Receiver State/Province
    state_province VARCHAR(40),

    -- A.1.5.6 - Receiver Postcode
    postcode VARCHAR(15),

    -- A.1.5.7 - Receiver Country Code
    country_code VARCHAR(2),  -- ISO 3166-1 alpha-2

    -- A.1.5.8 - Receiver Telephone
    telephone VARCHAR(33),

    -- A.1.5.9 - Receiver Fax
    fax VARCHAR(33),

    -- A.1.5.10 - Receiver Email
    email VARCHAR(100),

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_receiver_per_case UNIQUE (case_id)
);

CREATE INDEX idx_receiver_info_case ON receiver_information(case_id);

-- ============================================================================
-- C.1.9.r: Other Case Identifiers (Repeating)
-- Track additional identifiers from other sources (e.g., regulatory authority numbers)
-- ============================================================================

CREATE TABLE other_case_identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- C.1.9.1.r.1 - Source of the Case Identifier
    source_of_identifier VARCHAR(60) NOT NULL,

    -- C.1.9.1.r.2 - Case Identifier
    case_identifier VARCHAR(100) NOT NULL,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_other_identifier_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_other_case_identifiers_case ON other_case_identifiers(case_id);

-- ============================================================================
-- C.1.10.r: Linked Report Numbers (Repeating)
-- Links to follow-up reports, amendments, related cases
-- ============================================================================

CREATE TABLE linked_report_numbers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- C.1.10.r - Linked Report Number
    linked_report_number VARCHAR(100) NOT NULL,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_linked_report_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_linked_report_numbers_case ON linked_report_numbers(case_id);
