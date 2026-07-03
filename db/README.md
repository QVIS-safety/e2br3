This directory is the source of truth for database initialization.

- `admin/`: destructive admin-only setup helpers such as database recreation
- `bootstrap/`: base schema needed for a fresh database
- `migrations/`: ordered incremental schema/data changes when needed; currently empty after squash
- `seed/`: optional dev/demo seed data

Bootstrap files are ordered lexicographically. Keep each file scoped to a
domain or cross-cutting concern; avoid catch-all compatibility files that mix
unrelated tables, policies, triggers, and reference data.

Execution order:
1. `admin/00-recreate-db.sql` only when explicitly requested
2. `bootstrap/*.sql`
3. `migrations/*.sql`
4. `seed/*.sql` when seed is enabled
