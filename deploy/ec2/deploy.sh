#!/usr/bin/env sh
set -eu

APP_DIR="${APP_DIR:-/opt/e2br3}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.prod.yml}"
ENV_FILE="${ENV_FILE:-.env.prod}"
IMAGE_REF="${IMAGE_REF:-}"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BUNDLED_SCHEMAS_DIR="${SCRIPT_DIR}/schemas"

if [ -z "${IMAGE_REF}" ]; then
  echo "IMAGE_REF is required (for example ghcr.io/<owner>/e2br3-web-server:<sha>)"
  exit 1
fi

cd "${APP_DIR}"

if [ ! -f "${ENV_FILE}" ]; then
  echo "Missing ${APP_DIR}/${ENV_FILE}. Copy from .env.prod.example and fill secrets."
  exit 1
fi

if [ ! -f "${COMPOSE_FILE}" ]; then
  echo "Missing ${APP_DIR}/${COMPOSE_FILE}"
  exit 1
fi

# Load env file for preflight checks.
set -a
. "${ENV_FILE}"
set +a

SCHEMAS_DIR="${E2BR3_SCHEMAS_DIR:-${APP_DIR}/schemas}"
if [ -d "${BUNDLED_SCHEMAS_DIR}" ]; then
  echo "Syncing bundled schemas from ${BUNDLED_SCHEMAS_DIR} to ${SCHEMAS_DIR}"
  mkdir -p "${SCHEMAS_DIR}"
  cp -R "${BUNDLED_SCHEMAS_DIR}/." "${SCHEMAS_DIR}/"
fi

if [ ! -d "${SCHEMAS_DIR}/coreschemas" ] || [ ! -d "${SCHEMAS_DIR}/multicacheschemas" ]; then
  echo "Missing schema directories under ${SCHEMAS_DIR}."
  echo "Expected at least coreschemas/ and multicacheschemas/."
  exit 1
fi

if [ ! -f "${SCHEMAS_DIR}/multicacheschemas/MCCI_IN200100UV01.xsd" ] && \
   [ ! -f "${SCHEMAS_DIR}/MCCI_IN200100UV01.xsd" ]; then
  echo "Missing schema file under ${SCHEMAS_DIR}."
  echo "Expected MCCI_IN200100UV01.xsd (either at root or multicacheschemas/)."
  exit 1
fi

if [ -n "${GHCR_USERNAME:-}" ] && [ -n "${GHCR_TOKEN:-}" ]; then
  echo "${GHCR_TOKEN}" | docker login ghcr.io -u "${GHCR_USERNAME}" --password-stdin
fi

echo "Pulling ${IMAGE_REF}"
docker pull "${IMAGE_REF}"

if [ "${RESET_DB:-}" = "1" ]; then
  if [ -z "${SERVICE_DB_URL:-}" ]; then
    echo "SERVICE_DB_URL is required when RESET_DB=1"
    exit 1
  fi
  if [ -z "${SERVICE_DB_ROOT_URL:-}" ]; then
    echo "SERVICE_DB_ROOT_URL is required when RESET_DB=1"
    exit 1
  fi

  docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" stop app

  DATABASE_URL="${SERVICE_DB_URL}" \
  ROOT_DATABASE_URL="${SERVICE_DB_ROOT_URL}" \
  RESET_DB=1 \
  INCLUDE_SEED="${INCLUDE_SEED:-1}" \
  PROJECT_DIR="${APP_DIR}" \
  "${APP_DIR}/init-rds.sh"

  APP_DIR="${APP_DIR}" \
  ENV_FILE="${ENV_FILE}" \
  COMPOSE_FILE="${COMPOSE_FILE}" \
  E2BR3_TERMINOLOGY_DIR="${E2BR3_TERMINOLOGY_DIR:-/opt/e2br3/terminology}" \
  "${APP_DIR}/run-terminology-manifest.sh"
fi

# Update runtime image reference in env file idempotently.
if grep -q '^IMAGE_REF=' "${ENV_FILE}"; then
  sed -i.bak "s|^IMAGE_REF=.*|IMAGE_REF=${IMAGE_REF}|" "${ENV_FILE}"
else
  echo "IMAGE_REF=${IMAGE_REF}" >> "${ENV_FILE}"
fi

docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" up -d app

if [ -n "${HEALTHCHECK_URL:-}" ]; then
  attempt=1
  while [ "${attempt}" -le 10 ]; do
    if curl -fsS "${HEALTHCHECK_URL}" >/dev/null; then
      break
    fi
    if [ "${attempt}" -eq 10 ]; then
      echo "Healthcheck failed after 10 attempts: ${HEALTHCHECK_URL}"
      exit 1
    fi
    sleep 3
    attempt=$((attempt + 1))
  done
fi

docker image prune -f

echo "Deploy complete: ${IMAGE_REF}"
