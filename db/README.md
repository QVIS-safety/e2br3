# Frozen pre-release database

`bootstrap/*.sql` is the complete PostgreSQL 16 initial schema. Do not replay
historical migrations. Initialize an **empty** database with:

```sh
DATABASE_URL='<administrator connection>' APP_DB_USER='<runtime login>' \
  deploy/ec2/migrate-rds.sh --initialize
```

The historical command name is retained, but it only initializes or checks this
baseline. No arguments / `--check` verifies the recorded file checksums read-only.
All schema and reference-data files commit in one transaction; any error rolls
everything back. An existing database is never reset or adopted automatically.
`--seed` is explicit development demo data, not part of the production baseline.

## Order and provenance

1. `00-roles.sql`: guarded application/auditor/search roles and extensions.
2. `01-safetydb-schema.sql`: final tables, sequences, functions and column checks.
3. `02-reference-data.sql`: canonical code lists, UCUM units, defaults and the
   internal system principal (no login password). No customer records or keys.
4. `03-constraints.sql`: final keys, indexes, RLS policies and grants.
5. `09a-iso-countries.sql`, `11-fda-device-codes.sql`: authoritative reference data.
6. `12-triggers.sql`: final triggers, after reference-data loading.

The schema was captured from a full copy of the actual server database, upgraded
from its 135 recorded changes to the final 143-change state on 2026-11-27.
The original database was not modified. Canonical initial data came from the old
initializer **without demo seed**, not from customer rows. Licensed terminology
imports and organization AS2 credentials must still be supplied separately.

Server-specific `e2br3_app` login grants are not portable schema. Initialization
grants the caller-selected runtime login membership in `e2br3_app_role` and
`e2br3_auditor_role`, preserving the canonical role permissions. Object ownership
belongs to the initializer, except the narrowly privileged WHODrug search function,
which remains owned by `e2br3_whodrug_search_owner` (NOLOGIN, NOINHERIT, BYPASSRLS).

`bootstrap-inventory.json` records the restored catalog definitions. The actual
PostgreSQL test compares every inventoried object, not just table counts. The
live-copy comparison differed only in two equivalent PostgreSQL-rendered array
casts in authority checks; no schema object was missing. Capture with
`scripts/db/schema-inventory.sql`. This inventory is a regression assertion, not
a license to overwrite or mark an unverified existing database compatible.

The 143 runtime migration files were removed. Selected historical SQL under
`tests/fixtures/prebaseline/` is test input only and is excluded from runtime images.
After the first deployment, introduce explicit upgrade migrations for subsequent
schema changes; do not silently rewrite deployed baselines.

## Checks

```sh
python3 -m unittest discover -s deploy/ec2 -p 'test_migrations.py'
MIGRATION_TEST_ADMIN_URL='<administrator connection to postgres>' \
  python3 -m unittest discover -s deploy/ec2 -p 'test_migrations_postgres.py' -v
```

Existing development databases retain their data and old history. This change
does not automatically upgrade them. Create a new task database, or separately
review and verify an existing database before deliberate adoption.
