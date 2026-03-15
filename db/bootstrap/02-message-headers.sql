-- ============================================================================
-- SECTION N: Batch/Message Headers (Optional)
-- ============================================================================

CREATE TABLE if NOT EXISTS message_headers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,

    -- N.1.1 - Batch Number
    batch_number VARCHAR(100),

    -- N.1.2 - Batch Sender Identifier
    batch_sender_identifier VARCHAR(60),

    -- N.1.3 - Batch Receiver Identifier
    batch_receiver_identifier VARCHAR(60),

    -- N.1.4 - Date of Batch Transmission
    batch_transmission_date TIMESTAMPTZ,

    -- Message identification
    message_type VARCHAR(50) NOT NULL,              -- ichicsr
    message_format_version VARCHAR(10) NOT NULL,     -- 2.1
    message_format_release VARCHAR(10) NOT NULL,     -- 2.0
    message_number VARCHAR(100) UNIQUE NOT NULL,
    message_sender_identifier VARCHAR(60) NOT NULL,
    message_receiver_identifier VARCHAR(60) NOT NULL,
    message_date_format VARCHAR(10) NOT NULL,        -- 204 (CCYYMMDDHHMMSS)
    message_date VARCHAR(14) NOT NULL,

    -- Audit fields (standardized UUID-based)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_by UUID REFERENCES users(id) ON DELETE RESTRICT,

    CONSTRAINT unique_message_per_case UNIQUE (case_id)
);

CREATE INDEX idx_message_headers_case_id ON message_headers(case_id);
CREATE INDEX idx_message_headers_number ON message_headers(message_number);
