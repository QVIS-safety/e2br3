#!/usr/bin/env sh
set -eu

DB_DIR="${E2BR3_INIT_DB_DIR:-/db}"
INCLUDE_SEED="${E2BR3_INIT_INCLUDE_SEED:-0}"
export APP_DB_USER="${POSTGRES_USER:?POSTGRES_USER is required}"
export PGUSER="${POSTGRES_USER}"
export PGDATABASE="${POSTGRES_DB:?POSTGRES_DB is required}"
export PGHOST=/var/run/postgresql
case "${INCLUDE_SEED}" in
  0) set -- ;;
  1) set -- --seed ;;
  *) echo "E2BR3_INIT_INCLUDE_SEED must be 0 or 1" >&2; exit 1 ;;
esac
exec python3 /app-scripts/migrate-rds.py --db-dir "${DB_DIR}" --initialize "$@"
