#!/usr/bin/env sh
set -eu

# One-time schema/bootstrap loader for PostgreSQL RDS.
# Usage:
#   DATABASE_URL='postgres://user:pwd@host:5432/app_db?sslmode=require' ./deploy/ec2/init-rds.sh
# Optional:
#   INCLUDE_SEED=0 ./deploy/ec2/init-rds.sh   # skip db/seed/*.sql
#   RESET_DB=1 ROOT_DATABASE_URL='postgres://admin:pwd@host:5432/postgres?sslmode=require' \
#     DATABASE_URL='postgres://app_user:pwd@host:5432/app_db?sslmode=require' ./deploy/ec2/init-rds.sh
#   PROJECT_DIR=/path/to/repo ./deploy/ec2/init-rds.sh

DATABASE_URL="${DATABASE_URL:-}"
ROOT_DATABASE_URL="${ROOT_DATABASE_URL:-${SERVICE_DB_ROOT_URL:-}}"
PROJECT_DIR="${PROJECT_DIR:-$(pwd)}"
INCLUDE_SEED="${INCLUDE_SEED:-1}"
RESET_DB="${RESET_DB:-0}"
LIST_SQL_SCRIPT="${PROJECT_DIR}/scripts/db/list_init_sql.sh"
DB_DIR="${PROJECT_DIR}/db"

if [ -z "${DATABASE_URL}" ]; then
  echo "DATABASE_URL is required."
  echo "Example:"
  echo "  DATABASE_URL='postgres://user:pwd@host:5432/app_db?sslmode=require' ./deploy/ec2/init-rds.sh"
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required but not found on PATH."
  exit 1
fi

if [ ! -d "${DB_DIR}" ]; then
  echo "DB directory not found: ${DB_DIR}"
  echo "Set PROJECT_DIR to your repository root."
  exit 1
fi

if [ ! -f "${LIST_SQL_SCRIPT}" ]; then
  echo "SQL list helper not found: ${LIST_SQL_SCRIPT}"
  echo "Set PROJECT_DIR to your repository root."
  exit 1
fi

echo "Using DB directory: ${DB_DIR}"
if [ "${RESET_DB}" = "1" ]; then
  if [ -z "${ROOT_DATABASE_URL}" ]; then
    echo "ROOT_DATABASE_URL (or SERVICE_DB_ROOT_URL) is required when RESET_DB=1."
    echo "Example:"
    echo "  RESET_DB=1 ROOT_DATABASE_URL='postgres://admin:pwd@host:5432/postgres?sslmode=require' \\"
    echo "    DATABASE_URL='postgres://app_user:pwd@host:5432/app_db?sslmode=require' ./deploy/ec2/init-rds.sh"
    exit 1
  fi

  recreate_path="${DB_DIR}/admin/00-recreate-db.sql"
  if [ ! -f "${recreate_path}" ]; then
    echo "Missing file: ${recreate_path}"
    exit 1
  fi

  echo "RESET_DB=1 -> running admin/00-recreate-db.sql on root DB URL"
  psql "${ROOT_DATABASE_URL}" -v ON_ERROR_STOP=1 -f "${recreate_path}"
fi

echo "Applying SQL files to: ${DATABASE_URL}"

sh "${LIST_SQL_SCRIPT}" "${DB_DIR}" "${INCLUDE_SEED}" |
while IFS= read -r f; do
  path="${DB_DIR}/${f}"
  if [ ! -f "${path}" ]; then
    echo "Missing file: ${path}"
    exit 1
  fi
  echo "==> ${f}"
  psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -f "${path}"
done

echo "RDS bootstrap complete."
