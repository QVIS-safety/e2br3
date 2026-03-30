#!/usr/bin/env sh
set -eu

DB_DIR="${E2BR3_INIT_DB_DIR:-/db}"
LIST_SCRIPT="${E2BR3_INIT_SQL_LIST_SCRIPT:-/app-scripts/list_init_sql.sh}"
INCLUDE_SEED="${E2BR3_INIT_INCLUDE_SEED:-1}"

if [ ! -f "${LIST_SCRIPT}" ]; then
  echo "Missing SQL list helper: ${LIST_SCRIPT}"
  exit 1
fi

echo "Applying init SQL from ${DB_DIR}"
sh "${LIST_SCRIPT}" "${DB_DIR}" "${INCLUDE_SEED}" |
while IFS= read -r file; do
  echo "==> ${file}"
  psql -v ON_ERROR_STOP=1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -f "${DB_DIR}/${file}"
done
