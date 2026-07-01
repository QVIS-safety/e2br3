#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="${SCRIPT_DIR}/load-iso-countries.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

APP_DIR="${TMP_DIR}/app"
BIN_DIR="${TMP_DIR}/bin"
PSQL_LOG="${TMP_DIR}/psql.log"
CURL_LOG="${TMP_DIR}/curl.log"

mkdir -p "${APP_DIR}" "${BIN_DIR}"

cat > "${APP_DIR}/.env.prod" <<'ENV'
SERVICE_DB_URL=postgres://app_user:secret@example.com:5432/app_db?sslmode=require
ENV

cat > "${APP_DIR}/docker-compose.prod.yml" <<'YAML'
services:
  app:
    image: example/app
YAML

cat > "${BIN_DIR}/curl" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'CMD=%s\n' "$*" >> "${CURL_LOG}"
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o)
      shift
      out="${1:-}"
      ;;
  esac
  shift || true
done
if [ -z "${out}" ]; then
  echo "missing curl -o output path" >&2
  exit 1
fi
cat > "${out}" <<'CSV'
Name,Code
"Korea, Republic of",KR
United States,US
CSV
SH
chmod +x "${BIN_DIR}/curl"

cat > "${BIN_DIR}/psql" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'URL=%s\n' "$1" >> "${PSQL_LOG}"
sql_file=""
i=0
for arg in "$@"; do
  i=$((i + 1))
  printf 'ARG_%02d=%s\n' "${i}" "${arg}" >> "${PSQL_LOG}"
  if [ "${arg}" = "-f" ]; then
    shift
    sql_file="${1:-}"
    break
  fi
  shift || true
done
if [ -n "${sql_file}" ]; then
  cat "${sql_file}" >> "${PSQL_LOG}"
else
  cat >> "${PSQL_LOG}"
fi
SH
chmod +x "${BIN_DIR}/psql"

PATH="${BIN_DIR}:${PATH}" \
CURL_LOG="${CURL_LOG}" \
PSQL_LOG="${PSQL_LOG}" \
APP_DIR="${APP_DIR}" \
sh "${SCRIPT}"

grep -F "CMD=-fsSL -o " "${CURL_LOG}" >/dev/null
grep -F "https://datahub.io/core/country-list/_r/-/data.csv" "${CURL_LOG}" >/dev/null
grep -F "URL=postgres://app_user:secret@example.com:5432/app_db?sslmode=require" "${PSQL_LOG}" >/dev/null
grep -F "SELECT set_current_user_context('00000000-0000-0000-0000-000000000001'::uuid);" "${PSQL_LOG}" >/dev/null
grep -F "SELECT set_org_context('00000000-0000-0000-0000-000000000000'::uuid, 'system_admin');" "${PSQL_LOG}" >/dev/null
grep -F "INSERT INTO iso_countries (code, name, active)" "${PSQL_LOG}" >/dev/null
grep -F "ON CONFLICT (code)" "${PSQL_LOG}" >/dev/null
grep -F "UPDATE iso_countries AS country" "${PSQL_LOG}" >/dev/null
grep -F "source_url=https://datahub.io/core/country-list/_r/-/data.csv" "${PSQL_LOG}" >/dev/null
grep -F "\\copy staging_iso_countries(name, code) FROM '" "${PSQL_LOG}" >/dev/null
grep -F "Loaded ISO countries from :source_url" "${PSQL_LOG}" >/dev/null

if PATH="${BIN_DIR}:${PATH}" \
  CURL_LOG="${CURL_LOG}" \
  PSQL_LOG="${PSQL_LOG}" \
  APP_DIR="${APP_DIR}/missing" \
  sh "${SCRIPT}" 2>"${TMP_DIR}/missing-app.err"; then
  echo "script should fail when app env file is missing"
  exit 1
fi
grep -F "Missing env file:" "${TMP_DIR}/missing-app.err" >/dev/null

echo "load-iso-countries tests passed"
