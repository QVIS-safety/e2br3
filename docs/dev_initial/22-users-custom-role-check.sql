ALTER TABLE users DROP CONSTRAINT IF EXISTS user_role_valid;

ALTER TABLE users
    ADD CONSTRAINT user_role_valid CHECK (char_length(trim(role)) > 0);
