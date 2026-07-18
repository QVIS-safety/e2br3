CREATE TABLE IF NOT EXISTS rbac_policy_state (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    version bigint NOT NULL DEFAULT 1 CHECK (version > 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO rbac_policy_state (singleton, version)
VALUES (true, 1)
ON CONFLICT (singleton) DO NOTHING;

GRANT SELECT, UPDATE ON rbac_policy_state TO e2br3_app_role;
