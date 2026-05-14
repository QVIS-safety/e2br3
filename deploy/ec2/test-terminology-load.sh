#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="${SCRIPT_DIR}/terminology-load.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

APP_DIR="${TMP_DIR}/app"
INPUT_DIR="${APP_DIR}/terminology/incoming"
BIN_LOG="${TMP_DIR}/loader.log"
mkdir -p "${INPUT_DIR}" "${TMP_DIR}/bin"

cat > "${APP_DIR}/.env.prod" <<'ENV'
SERVICE_DB_URL=postgres://app_user:secret@example.com:5432/app_db?sslmode=require
ENV

cat > "${TMP_DIR}/bin/fake-terminology-loader" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'SERVICE_DB_URL=%s\n' "${SERVICE_DB_URL:-}" >> "${TERMINOLOGY_LOADER_LOG}"
printf 'ARGS=%s\n' "$*" >> "${TERMINOLOGY_LOADER_LOG}"
SH
chmod +x "${TMP_DIR}/bin/fake-terminology-loader"

touch "${INPUT_DIR}/meddra.zip" "${INPUT_DIR}/whodrug.zip"

APP_DIR="${APP_DIR}" \
TERMINOLOGY_LOADER_BIN="${TMP_DIR}/bin/fake-terminology-loader" \
TERMINOLOGY_LOADER_LOG="${BIN_LOG}" \
sh "${SCRIPT}" --dry-run meddra "${INPUT_DIR}/meddra.zip" 27.1

grep -F "SERVICE_DB_URL=postgres://app_user:secret@example.com:5432/app_db?sslmode=require" "${BIN_LOG}" >/dev/null
grep -F "ARGS=meddra --input ${INPUT_DIR}/meddra.zip --version 27.1 --language en --dry-run" "${BIN_LOG}" >/dev/null

: > "${BIN_LOG}"
APP_DIR="${APP_DIR}" \
TERMINOLOGY_LOADER_BIN="${TMP_DIR}/bin/fake-terminology-loader" \
TERMINOLOGY_LOADER_LOG="${BIN_LOG}" \
sh "${SCRIPT}" --load whodrug "${INPUT_DIR}/whodrug.zip" 2025.09 ko

grep -F "ARGS=whodrug --input ${INPUT_DIR}/whodrug.zip --version 2025.09 --language ko" "${BIN_LOG}" >/dev/null
if grep -F -- "--dry-run" "${BIN_LOG}" >/dev/null; then
  echo "load mode must not pass --dry-run"
  exit 1
fi

if APP_DIR="${APP_DIR}" sh "${SCRIPT}" meddra "${INPUT_DIR}/meddra.zip" 27.1 2>"${TMP_DIR}/missing-mode.err"; then
  echo "script should require --dry-run or --load"
  exit 1
fi
grep -F "Choose exactly one mode: --dry-run or --load." "${TMP_DIR}/missing-mode.err" >/dev/null

echo "terminology-load.sh tests passed."
