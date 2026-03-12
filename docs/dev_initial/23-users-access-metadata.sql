ALTER TABLE users
    ADD COLUMN IF NOT EXISTS comments TEXT,
    ADD COLUMN IF NOT EXISTS other_information TEXT,
    ADD COLUMN IF NOT EXISTS access_start_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS access_end_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS access_sender_ids TEXT,
    ADD COLUMN IF NOT EXISTS access_product_ids TEXT,
    ADD COLUMN IF NOT EXISTS access_study_ids TEXT;
