#!/usr/bin/env sh
set -eu

APP_DIR="${APP_DIR:-/opt/e2br3}"
ENV_FILE="${ENV_FILE:-.env.prod}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.prod.yml}"
E2BR3_TERMINOLOGY_DIR="${E2BR3_TERMINOLOGY_DIR:-/opt/e2br3/terminology}"

usage() {
  cat >&2 <<'EOF'
Usage:
  terminology-load.sh --dry-run <meddra|whodrug> <input-path> <version> [language]
  terminology-load.sh --load    <meddra|whodrug> <input-path> <version> [language]

Environment:
  APP_DIR                  EC2 app directory. Default: /opt/e2br3
  ENV_FILE                 Docker Compose env file, relative to APP_DIR unless absolute. Default: .env.prod
  COMPOSE_FILE             Docker Compose file, relative to APP_DIR unless absolute. Default: docker-compose.prod.yml
  E2BR3_TERMINOLOGY_DIR    Host terminology mount root. Default: /opt/e2br3/terminology

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

case "${COMPOSE_FILE}" in
  /*) RESOLVED_COMPOSE_FILE="${COMPOSE_FILE}" ;;
  *) RESOLVED_COMPOSE_FILE="${APP_DIR}/${COMPOSE_FILE}" ;;
esac

if [ ! -f "${RESOLVED_ENV_FILE}" ]; then
  echo "Missing env file: ${RESOLVED_ENV_FILE}" >&2
  exit 1
fi

if [ ! -f "${RESOLVED_COMPOSE_FILE}" ]; then
  echo "Missing compose file: ${RESOLVED_COMPOSE_FILE}" >&2
  exit 1
fi

INPUT_DIR=$(CDPATH= cd -- "$(dirname -- "${INPUT_PATH}")" && pwd)
INPUT_ABS="${INPUT_DIR}/$(basename -- "${INPUT_PATH}")"
TERMINOLOGY_ABS=$(CDPATH= cd -- "${E2BR3_TERMINOLOGY_DIR}" && pwd)

case "${INPUT_ABS}" in
  "${TERMINOLOGY_ABS}"/*)
    RELATIVE_INPUT=${INPUT_ABS#"${TERMINOLOGY_ABS}/"}
    ;;
  *)
    echo "Input path must be under E2BR3_TERMINOLOGY_DIR: ${E2BR3_TERMINOLOGY_DIR}" >&2
    exit 1
    ;;
esac

CONTAINER_INPUT="/terminology/${RELATIVE_INPUT}"

echo "Dictionary: ${DICTIONARY}"
echo "Input: ${INPUT_ABS}"
echo "Container input: ${CONTAINER_INPUT}"
echo "Version: ${VERSION}"
echo "Language: ${LANGUAGE}"
echo "Mode: ${MODE}"

cd "${APP_DIR}"

set -- docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" run --rm terminology-loader \
  "${DICTIONARY}" \
  --input "${CONTAINER_INPUT}" \
  --version "${VERSION}" \
  --language "${LANGUAGE}"

if [ "${MODE}" = "dry-run" ]; then
  set -- "$@" --dry-run
fi

echo "Starting terminology ${MODE}..."
"$@"
echo "Terminology ${MODE} complete."
