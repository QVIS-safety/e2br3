#!/usr/bin/env sh
set -eu

APP_DIR="${APP_DIR:-/opt/e2br3}"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
COMPOSE_FILE="${COMPOSE_FILE:-${SCRIPT_DIR}/docker-compose.prod.yml}"
ENV_FILE="${ENV_FILE:-${APP_DIR}/.env.prod}"
IMAGE_REF="${IMAGE_REF:-}"
REQUESTED_IMAGE_REF="${IMAGE_REF}"
REQUESTED_RESET_DB_SET=${RESET_DB+x}
REQUESTED_RESET_DB=${RESET_DB:-}
REQUESTED_INCLUDE_SEED_SET=${INCLUDE_SEED+x}
REQUESTED_INCLUDE_SEED=${INCLUDE_SEED:-}
REQUESTED_RESET_PRESERVE_TERMINOLOGY_SET=${RESET_PRESERVE_TERMINOLOGY+x}
REQUESTED_RESET_PRESERVE_TERMINOLOGY=${RESET_PRESERVE_TERMINOLOGY:-}
REQUESTED_RELOAD_TERMINOLOGY_SET=${RELOAD_TERMINOLOGY+x}
REQUESTED_RELOAD_TERMINOLOGY=${RELOAD_TERMINOLOGY:-}
REQUESTED_HEALTHCHECK_URL_SET=${HEALTHCHECK_URL+x}
REQUESTED_HEALTHCHECK_URL=${HEALTHCHECK_URL:-}
INIT_RDS_SCRIPT="${INIT_RDS_SCRIPT:-${SCRIPT_DIR}/init-rds.sh}"
TERMINOLOGY_MANIFEST_SCRIPT="${TERMINOLOGY_MANIFEST_SCRIPT:-${SCRIPT_DIR}/run-terminology-manifest.sh}"
ISO_COUNTRIES_SCRIPT="${ISO_COUNTRIES_SCRIPT:-${SCRIPT_DIR}/load-iso-countries.sh}"
if [ -d "${SCRIPT_DIR}/../../docs/exporter/schema" ]; then
  BUNDLED_SCHEMAS_DIR="${SCRIPT_DIR}/../../docs/exporter/schema"
else
  BUNDLED_SCHEMAS_DIR="${SCRIPT_DIR}/schemas"
fi

if [ -z "${IMAGE_REF}" ]; then
  echo "IMAGE_REF is required (for example ghcr.io/<owner>/e2br3-web-server:<sha>)"
  exit 1
fi

cd "${APP_DIR}"

if [ ! -f "${ENV_FILE}" ]; then
  echo "Missing ${ENV_FILE}. Create it with the required runtime secrets and database settings."
  exit 1
fi

if [ ! -f "${COMPOSE_FILE}" ]; then
  echo "Missing ${COMPOSE_FILE}"
  exit 1
fi

# Load env file for preflight checks.
case "${ENV_FILE}" in
  */*) ENV_FILE_SOURCE="${ENV_FILE}" ;;
  *) ENV_FILE_SOURCE="./${ENV_FILE}" ;;
esac
set -a
. "${ENV_FILE_SOURCE}"
set +a
IMAGE_REF="${REQUESTED_IMAGE_REF}"
if [ -n "${REQUESTED_RESET_DB_SET}" ]; then
  RESET_DB="${REQUESTED_RESET_DB}"
fi
if [ -n "${REQUESTED_INCLUDE_SEED_SET}" ]; then
  INCLUDE_SEED="${REQUESTED_INCLUDE_SEED}"
fi
if [ -n "${REQUESTED_RESET_PRESERVE_TERMINOLOGY_SET}" ]; then
  RESET_PRESERVE_TERMINOLOGY="${REQUESTED_RESET_PRESERVE_TERMINOLOGY}"
fi
if [ -n "${REQUESTED_RELOAD_TERMINOLOGY_SET}" ]; then
  RELOAD_TERMINOLOGY="${REQUESTED_RELOAD_TERMINOLOGY}"
fi
if [ -n "${REQUESTED_HEALTHCHECK_URL_SET}" ]; then
  HEALTHCHECK_URL="${REQUESTED_HEALTHCHECK_URL}"
fi
RESET_PRESERVE_TERMINOLOGY="${RESET_PRESERVE_TERMINOLOGY:-1}"
RELOAD_TERMINOLOGY="${RELOAD_TERMINOLOGY:-0}"
LOAD_ISO_COUNTRIES="${LOAD_ISO_COUNTRIES:-1}"

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
  if ! printf '%s' "${GHCR_TOKEN}" | docker login ghcr.io -u "${GHCR_USERNAME}" --password-stdin; then
    echo "Warning: GHCR login failed; continuing and letting docker pull verify image access." >&2
  fi
fi

echo "Pulling ${IMAGE_REF}"
docker pull "${IMAGE_REF}"

had_previous_image_ref=0
previous_image_ref=
if grep -q '^IMAGE_REF=' "${ENV_FILE}"; then
  had_previous_image_ref=1
  previous_image_ref=$(sed -n 's/^IMAGE_REF=//p' "${ENV_FILE}" | sed -n '1p')
fi

restore_previous_image_ref() {
  if [ "${had_previous_image_ref}" -eq 1 ]; then
    IMAGE_REF="${previous_image_ref}"
    export IMAGE_REF
    if grep -q '^IMAGE_REF=' "${ENV_FILE}"; then
      sed -i.bak "s|^IMAGE_REF=.*|IMAGE_REF=${previous_image_ref}|" "${ENV_FILE}"
    else
      echo "IMAGE_REF=${previous_image_ref}" >> "${ENV_FILE}"
    fi
  else
    unset IMAGE_REF
    sed -i.bak '/^IMAGE_REF=/d' "${ENV_FILE}"
  fi
}

if [ "${RESET_DB:-}" = "1" ]; then
  if [ -z "${SERVICE_DB_URL:-}" ]; then
    echo "SERVICE_DB_URL is required when RESET_DB=1"
    exit 1
  fi
  if [ -z "${SERVICE_DB_ROOT_URL:-}" ]; then
    echo "SERVICE_DB_ROOT_URL is required when RESET_DB=1"
    exit 1
  fi

  if [ "${RELOAD_TERMINOLOGY}" = "1" ]; then
    CHECK_ONLY=1 \
    APP_DIR="${APP_DIR}" \
    ENV_FILE="${ENV_FILE}" \
    COMPOSE_FILE="${COMPOSE_FILE}" \
    E2BR3_TERMINOLOGY_DIR="${E2BR3_TERMINOLOGY_DIR:-/opt/e2br3/terminology}" \
    "${TERMINOLOGY_MANIFEST_SCRIPT}"
  fi

  docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" stop app

  DATABASE_URL="${SERVICE_DB_URL}" \
  ROOT_DATABASE_URL="${SERVICE_DB_ROOT_URL}" \
  RESET_DB=1 \
  RESET_PRESERVE_TERMINOLOGY="${RESET_PRESERVE_TERMINOLOGY}" \
  INCLUDE_SEED="${INCLUDE_SEED:-1}" \
  PROJECT_DIR="${APP_DIR}" \
  "${INIT_RDS_SCRIPT}"

  if [ "${RELOAD_TERMINOLOGY}" = "1" ]; then
    APP_DIR="${APP_DIR}" \
    ENV_FILE="${ENV_FILE}" \
    COMPOSE_FILE="${COMPOSE_FILE}" \
    E2BR3_TERMINOLOGY_DIR="${E2BR3_TERMINOLOGY_DIR:-/opt/e2br3/terminology}" \
    "${TERMINOLOGY_MANIFEST_SCRIPT}"
  fi

  if [ "${LOAD_ISO_COUNTRIES}" = "1" ]; then
    APP_DIR="${APP_DIR}" \
    ENV_FILE="${ENV_FILE}" \
    "${ISO_COUNTRIES_SCRIPT}"
  fi
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
      restore_previous_image_ref
      docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" up -d app
      exit 1
    fi
    sleep 3
    attempt=$((attempt + 1))
  done
fi

docker image prune -f

echo "Deploy complete: ${IMAGE_REF}"
