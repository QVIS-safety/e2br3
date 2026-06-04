#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="${SCRIPT_DIR}/terminology-load.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

APP_DIR="${TMP_DIR}/app"
BIN_DIR="${TMP_DIR}/bin"
INPUT_DIR="${TMP_DIR}/terminology/incoming"
LOG="${TMP_DIR}/docker.log"

mkdir -p "${APP_DIR}" "${BIN_DIR}" "${INPUT_DIR}"

cat > "${APP_DIR}/.env.prod" <<'ENV'
SERVICE_DB_URL=postgres://app_user:secret@example.com:5432/app_db?sslmode=require
ENV

cat > "${APP_DIR}/docker-compose.prod.yml" <<'YAML'
services:
  terminology-loader:
    image: example/terminology-loader
YAML

cat > "${BIN_DIR}/docker" <<'SH'
#!/usr/bin/env sh
set -eu
i=0
for arg in "$@"; do
  i=$((i + 1))
  printf 'ARG_%02d=%s\n' "${i}" "${arg}" >> "${DOCKER_LOG}"
done
printf 'CMD=%s\n' "$*" >> "${DOCKER_LOG}"
SH
chmod +x "${BIN_DIR}/docker"

touch "${INPUT_DIR}/meddra.zip" "${INPUT_DIR}/whodrug.zip"

PATH="${BIN_DIR}:${PATH}" \
DOCKER_LOG="${LOG}" \
APP_DIR="${APP_DIR}" \
E2BR3_TERMINOLOGY_DIR="${TMP_DIR}/terminology" \
sh "${SCRIPT}" --dry-run meddra "${INPUT_DIR}/meddra.zip" 27.1

grep -F "CMD=compose --env-file .env.prod -f docker-compose.prod.yml run --rm terminology-loader meddra --input /terminology/incoming/meddra.zip --version 27.1 --language en --dry-run" "${LOG}" >/dev/null
grep -F "ARG_11=/terminology/incoming/meddra.zip" "${LOG}" >/dev/null
grep -F "ARG_16=--dry-run" "${LOG}" >/dev/null

: > "${LOG}"
PATH="${BIN_DIR}:${PATH}" \
DOCKER_LOG="${LOG}" \
APP_DIR="${APP_DIR}" \
E2BR3_TERMINOLOGY_DIR="${TMP_DIR}/terminology" \
sh "${SCRIPT}" --load whodrug "${INPUT_DIR}/whodrug.zip" 2025.09 ko

grep -F "CMD=compose --env-file .env.prod -f docker-compose.prod.yml run --rm terminology-loader whodrug --input /terminology/incoming/whodrug.zip --version 2025.09 --language ko" "${LOG}" >/dev/null
grep -F "ARG_11=/terminology/incoming/whodrug.zip" "${LOG}" >/dev/null
if grep -F -- "--dry-run" "${LOG}" >/dev/null; then
  echo "load mode must not pass --dry-run"
  exit 1
fi

if PATH="${BIN_DIR}:${PATH}" \
  DOCKER_LOG="${LOG}" \
  APP_DIR="${APP_DIR}" \
  E2BR3_TERMINOLOGY_DIR="${TMP_DIR}/terminology" \
  sh "${SCRIPT}" --load meddra "${INPUT_DIR}/missing.zip" 27.1 2>"${TMP_DIR}/missing-input.err"; then
  echo "script should fail when input is missing"
  exit 1
fi
grep -F "Input path not found: ${INPUT_DIR}/missing.zip" "${TMP_DIR}/missing-input.err" >/dev/null

echo "terminology-load tests passed"
