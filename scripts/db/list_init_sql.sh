#!/usr/bin/env sh
set -eu

DB_DIR="${1:-}"
INCLUDE_SEED="${2:-1}"

if [ -z "${DB_DIR}" ]; then
  echo "Usage: $0 <db_dir> [include_seed]" >&2
  exit 1
fi

if [ ! -d "${DB_DIR}" ]; then
  echo "DB directory not found: ${DB_DIR}" >&2
  exit 1
fi

list_group() {
  group_dir="${DB_DIR}/$1"
  group_name="$1"

  if [ ! -d "${group_dir}" ]; then
    return
  fi

  LC_ALL=C find "${group_dir}" -maxdepth 1 -type f -name '*.sql' -exec basename {} \; | sort |
  while IFS= read -r file; do
    printf '%s/%s\n' "${group_name}" "${file}"
  done
}

list_group "bootstrap"
list_group "migrations"

if [ "${INCLUDE_SEED}" = "1" ]; then
  list_group "seed"
fi
