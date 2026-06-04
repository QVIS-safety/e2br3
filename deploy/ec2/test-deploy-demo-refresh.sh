#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="${SCRIPT_DIR}/deploy.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

APP_DIR="${TMP_DIR}/app"
BIN_DIR="${TMP_DIR}/bin"
DEPLOY_LOG="${TMP_DIR}/deploy.log"

mkdir -p \
  "${APP_DIR}/schemas/coreschemas" \
  "${APP_DIR}/schemas/multicacheschemas" \
  "${BIN_DIR}"
touch "${APP_DIR}/schemas/multicacheschemas/MCCI_IN200100UV01.xsd"

cat > "${APP_DIR}/.env.prod" <<'ENV'
SERVICE_DB_URL=postgres://app_user:pwd@example/app_db
SERVICE_DB_ROOT_URL=postgres://root:pwd@example/postgres
SERVICE_PWD_KEY=test-pwd-key
SERVICE_TOKEN_KEY=test-token-key
E2BR3_SCHEMAS_DIR=schemas
ENV

cat > "${APP_DIR}/docker-compose.prod.yml" <<'YAML'
services:
  app:
    image: placeholder
YAML

cat > "${APP_DIR}/init-rds.sh" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'init-rds RESET_DB=%s INCLUDE_SEED=%s DATABASE_URL=%s ROOT_DATABASE_URL=%s\n' \
  "${RESET_DB:-}" \
  "${INCLUDE_SEED:-}" \
  "${DATABASE_URL:-}" \
  "${ROOT_DATABASE_URL:-}" >> "${DEPLOY_LOG}"
SH
chmod +x "${APP_DIR}/init-rds.sh"

cat > "${APP_DIR}/run-terminology-manifest.sh" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'terminology manifest\n' >> "${DEPLOY_LOG}"
SH
chmod +x "${APP_DIR}/run-terminology-manifest.sh"

cat > "${BIN_DIR}/docker" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'docker %s\n' "$*" >> "${DEPLOY_LOG}"
SH
chmod +x "${BIN_DIR}/docker"

PATH="${BIN_DIR}:${PATH}" \
DEPLOY_LOG="${DEPLOY_LOG}" \
APP_DIR="${APP_DIR}" \
COMPOSE_FILE=docker-compose.prod.yml \
ENV_FILE=.env.prod \
IMAGE_REF=ghcr.io/example/e2br3-web-server:abc123 \
RESET_DB=1 \
INCLUDE_SEED=1 \
HEALTHCHECK_URL="" \
sh "${SCRIPT}"

cat > "${TMP_DIR}/expected.log" <<'LOG'
docker pull ghcr.io/example/e2br3-web-server:abc123
docker compose --env-file .env.prod -f docker-compose.prod.yml stop app
init-rds RESET_DB=1 INCLUDE_SEED=1 DATABASE_URL=postgres://app_user:pwd@example/app_db ROOT_DATABASE_URL=postgres://root:pwd@example/postgres
terminology manifest
docker compose --env-file .env.prod -f docker-compose.prod.yml up -d app
docker image prune -f
LOG

if ! cmp -s "${TMP_DIR}/expected.log" "${DEPLOY_LOG}"; then
  echo "unexpected deploy log"
  diff -u "${TMP_DIR}/expected.log" "${DEPLOY_LOG}" || true
  exit 1
fi

grep -F "IMAGE_REF=ghcr.io/example/e2br3-web-server:abc123" "${APP_DIR}/.env.prod" >/dev/null

echo "deploy demo refresh test passed"
