CREATE TABLE IF NOT EXISTS app_roles (
    role_name text PRIMARY KEY,
    display_name text NOT NULL,
    can_view boolean NOT NULL DEFAULT true,
    can_review boolean NOT NULL DEFAULT false,
    can_lock boolean NOT NULL DEFAULT false,
    can_admin boolean NOT NULL DEFAULT false,
    active boolean NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now()
);
