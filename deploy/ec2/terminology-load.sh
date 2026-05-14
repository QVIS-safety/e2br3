#!/usr/bin/env sh
set -eu

APP_DIR="${APP_DIR:-/opt/e2br3}"
ENV_FILE="${ENV_FILE:-.env.prod}"
PROJECT_DIR="${PROJECT_DIR:-${APP_DIR}/e2br3}"
TERMINOLOGY_LOADER_BIN="${TERMINOLOGY_LOADER_BIN:-}"

usage() {
  cat >&2 <<'EOF'
Usage:
  terminology-load.sh --dry-run <meddra|whodrug> <input-path> <version> [language]
  terminology-load.sh --load    <meddra|whodrug> <input-path> <version> [language]

Environment:
  APP_DIR                 EC2 app directory. Default: /opt/e2br3
  ENV_FILE                Env file path, relative to APP_DIR unless absolute. Default: .env.prod
  PROJECT_DIR             Repo checkout used for cargo run. Default: ${APP_DIR}/e2br3
  TERMINOLOGY_LOADER_BIN  Optional prebuilt terminology-loader binary path.

Examples:
  APP_DIR=/opt/e2br3 ./terminology-load.sh --dry-run meddra /opt/e2br3/terminology/incoming/meddra_27_1.zip 27.1
  APP_DIR=/opt/e2br3 ./terminology-load.sh --load whodrug /opt/e2br3/terminology/incoming/whodrug_2025_09.zip 2025.09 en
EOF
}

MODE=""
if [ "${1:-}" = "--dry-run" ]; then
  MODE="dry-run"
  shift
elif [ "${1:-}" = "--load" ]; then
  MODE="load"
  shift
fi

if [ -z "${MODE}" ]; then
  echo "Choose exactly one mode: --dry-run or --load." >&2
  usage
  exit 2
fi

DICTIONARY="${1:-}"
INPUT_PATH="${2:-}"
VERSION="${3:-}"
LANGUAGE="${4:-en}"

if [ -z "${DICTIONARY}" ] || [ -z "${INPUT_PATH}" ] || [ -z "${VERSION}" ]; then
  usage
  exit 2
fi

case "${DICTIONARY}" in
  meddra|whodrug) ;;
  *)
    echo "Unsupported dictionary: ${DICTIONARY}. Expected meddra or whodrug." >&2
    exit 2
    ;;
esac

if [ ! -e "${INPUT_PATH}" ]; then
  echo "Input path not found: ${INPUT_PATH}" >&2
  exit 1
fi

case "${ENV_FILE}" in
  /*) RESOLVED_ENV_FILE="${ENV_FILE}" ;;
  *) RESOLVED_ENV_FILE="${APP_DIR}/${ENV_FILE}" ;;
esac

if [ ! -f "${RESOLVED_ENV_FILE}" ]; then
  echo "Missing env file: ${RESOLVED_ENV_FILE}" >&2
  echo "Set APP_DIR or ENV_FILE to the EC2 production env file." >&2
  exit 1
fi

set -a
. "${RESOLVED_ENV_FILE}"
set +a

if [ -z "${SERVICE_DB_URL:-}" ]; then
  echo "SERVICE_DB_URL is required in ${RESOLVED_ENV_FILE}." >&2
  exit 1
fi

echo "Dictionary: ${DICTIONARY}"
echo "Input: ${INPUT_PATH}"
echo "Version: ${VERSION}"
echo "Language: ${LANGUAGE}"
echo "Mode: ${MODE}"

if [ -n "${TERMINOLOGY_LOADER_BIN}" ]; then
  if [ ! -x "${TERMINOLOGY_LOADER_BIN}" ]; then
    echo "TERMINOLOGY_LOADER_BIN is not executable: ${TERMINOLOGY_LOADER_BIN}" >&2
    exit 1
  fi
  set -- "${TERMINOLOGY_LOADER_BIN}" "${DICTIONARY}" --input "${INPUT_PATH}" --version "${VERSION}" --language "${LANGUAGE}"
else
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required when TERMINOLOGY_LOADER_BIN is not set." >&2
    echo "Install Rust on EC2 or provide a prebuilt terminology-loader binary." >&2
    exit 1
  fi
  if [ ! -f "${PROJECT_DIR}/Cargo.toml" ]; then
    echo "Cargo.toml not found under PROJECT_DIR: ${PROJECT_DIR}" >&2
    echo "Set PROJECT_DIR to the repository checkout on EC2." >&2
    exit 1
  fi
  set -- cargo run --manifest-path "${PROJECT_DIR}/Cargo.toml" -p terminology-loader -- "${DICTIONARY}" --input "${INPUT_PATH}" --version "${VERSION}" --language "${LANGUAGE}"
fi

if [ "${MODE}" = "dry-run" ]; then
  set -- "$@" --dry-run
fi

echo "Starting terminology ${MODE}..."
"$@"
echo "Terminology ${MODE} complete."
