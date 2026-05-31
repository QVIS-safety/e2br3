-- ============================================================================
-- SECTION F: Tests and Procedures (Repeating)
-- ============================================================================

CREATE TABLE test_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    sequence_number INTEGER NOT NULL,

    -- F.r.1 - Test Date
    test_date DATE,
    test_date_null_flavor VARCHAR(10),

    -- F.r.2 - Test Name (free text or MedDRA coded)
    test_name VARCHAR(250) NOT NULL,

    -- F.r.2.1 - Test Name (MedDRA coded)
    test_meddra_version VARCHAR(10),
    test_meddra_code VARCHAR(20),

    -- F.r.3.1 - Test Result (coded)
    test_result_code VARCHAR(2),  -- E2B(R3) code list

    -- F.r.3.2 - Test Result (value/finding)
    test_result_value VARCHAR(20000),  -- Can be numeric or text

    -- F.r.3.3 - Test Result Unit
    test_result_unit VARCHAR(50),

    -- F.r.3.4 - Result Unstructured Data
    result_unstructured TEXT,

    -- F.r.4 - Normal Low Value
    normal_low_value VARCHAR(50),

    -- F.r.5 - Normal High Value
    normal_high_value VARCHAR(50),

    -- F.r.6 - Comments
    comments TEXT,

    -- F.r.7 - More Information Available
    more_info_available BOOLEAN,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_test_result_sequence UNIQUE (case_id, sequence_number)
);

CREATE INDEX idx_test_results_case ON test_results(case_id);
