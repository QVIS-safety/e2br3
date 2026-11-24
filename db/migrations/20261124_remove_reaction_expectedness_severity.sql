ALTER TABLE reactions
    DROP COLUMN IF EXISTS expectedness,
    DROP COLUMN IF EXISTS severity;
