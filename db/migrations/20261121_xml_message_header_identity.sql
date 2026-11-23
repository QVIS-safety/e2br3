-- External message identifiers can recur across organizations and case versions.
-- Keep the per-case unique constraint, lookup index and RLS policies intact.
ALTER TABLE message_headers DROP CONSTRAINT message_headers_message_number_key;
ALTER TABLE message_headers ALTER COLUMN message_date TYPE TEXT;
