#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="${SCRIPT_DIR}/deploy.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

BIN_DIR="${TMP_DIR}/bin"
mkdir -p "${BIN_DIR}"

create_app() {
  APP_DIR=$1
  mkdir -p \
    "${APP_DIR}/schemas/coreschemas" \
    "${APP_DIR}/schemas/multicacheschemas"
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
printf 'init-rds RESET_DB=%s RESET_PRESERVE_TERMINOLOGY=%s INCLUDE_SEED=%s DATABASE_URL=%s ROOT_DATABASE_URL=%s\n' \
  "${RESET_DB:-}" \
  "${RESET_PRESERVE_TERMINOLOGY:-}" \
  "${INCLUDE_SEED:-}" \
  "${DATABASE_URL:-}" \
  "${ROOT_DATABASE_URL:-}" >> "${DEPLOY_LOG}"
SH
  chmod +x "${APP_DIR}/init-rds.sh"

  INIT_RDS_SCRIPT="${APP_DIR}/init-rds.sh"
  TERMINOLOGY_MANIFEST_SCRIPT="${APP_DIR}/run-terminology-manifest.sh"
  export INIT_RDS_SCRIPT TERMINOLOGY_MANIFEST_SCRIPT
}

APP_DIR="${TMP_DIR}/app-success"
DEPLOY_LOG="${TMP_DIR}/deploy-success.log"
create_app "${APP_DIR}"

cat > "${APP_DIR}/run-terminology-manifest.sh" <<'SH'
#!/usr/bin/env sh
set -eu
if [ "${CHECK_ONLY:-}" != "1" ]; then
  printf 'terminology manifest\n' >> "${DEPLOY_LOG}"
fi
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
init-rds RESET_DB=1 RESET_PRESERVE_TERMINOLOGY=1 INCLUDE_SEED=1 DATABASE_URL=postgres://app_user:pwd@example/app_db ROOT_DATABASE_URL=postgres://root:pwd@example/postgres
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

APP_DIR="${TMP_DIR}/app-requested-reset"
DEPLOY_LOG="${TMP_DIR}/deploy-requested-reset.log"
create_app "${APP_DIR}"
cat >> "${APP_DIR}/.env.prod" <<'ENV'
RESET_DB=0
INCLUDE_SEED=0
ENV

cat > "${APP_DIR}/run-terminology-manifest.sh" <<'SH'
#!/usr/bin/env sh
set -eu
if [ "${CHECK_ONLY:-}" != "1" ]; then
  printf 'terminology manifest\n' >> "${DEPLOY_LOG}"
fi
SH
chmod +x "${APP_DIR}/run-terminology-manifest.sh"

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

cat > "${TMP_DIR}/expected-requested-reset.log" <<'LOG'
docker pull ghcr.io/example/e2br3-web-server:abc123
docker compose --env-file .env.prod -f docker-compose.prod.yml stop app
init-rds RESET_DB=1 RESET_PRESERVE_TERMINOLOGY=1 INCLUDE_SEED=1 DATABASE_URL=postgres://app_user:pwd@example/app_db ROOT_DATABASE_URL=postgres://root:pwd@example/postgres
docker compose --env-file .env.prod -f docker-compose.prod.yml up -d app
docker image prune -f
LOG

if ! cmp -s "${TMP_DIR}/expected-requested-reset.log" "${DEPLOY_LOG}"; then
  echo "caller RESET_DB and INCLUDE_SEED should override .env.prod"
  diff -u "${TMP_DIR}/expected-requested-reset.log" "${DEPLOY_LOG}" || true
  exit 1
fi

APP_DIR="${TMP_DIR}/app-reload-terminology"
DEPLOY_LOG="${TMP_DIR}/deploy-reload-terminology.log"
create_app "${APP_DIR}"

cat > "${APP_DIR}/run-terminology-manifest.sh" <<'SH'
#!/usr/bin/env sh
set -eu
if [ "${CHECK_ONLY:-}" = "1" ]; then
  printf 'terminology preflight\n' >> "${DEPLOY_LOG}"
else
  printf 'terminology manifest\n' >> "${DEPLOY_LOG}"
fi
SH
chmod +x "${APP_DIR}/run-terminology-manifest.sh"

PATH="${BIN_DIR}:${PATH}" \
DEPLOY_LOG="${DEPLOY_LOG}" \
APP_DIR="${APP_DIR}" \
COMPOSE_FILE=docker-compose.prod.yml \
ENV_FILE=.env.prod \
IMAGE_REF=ghcr.io/example/e2br3-web-server:abc123 \
RESET_DB=1 \
INCLUDE_SEED=1 \
RELOAD_TERMINOLOGY=1 \
HEALTHCHECK_URL="" \
sh "${SCRIPT}"

cat > "${TMP_DIR}/expected-reload-terminology.log" <<'LOG'
docker pull ghcr.io/example/e2br3-web-server:abc123
terminology preflight
docker compose --env-file .env.prod -f docker-compose.prod.yml stop app
init-rds RESET_DB=1 RESET_PRESERVE_TERMINOLOGY=1 INCLUDE_SEED=1 DATABASE_URL=postgres://app_user:pwd@example/app_db ROOT_DATABASE_URL=postgres://root:pwd@example/postgres
terminology manifest
docker compose --env-file .env.prod -f docker-compose.prod.yml up -d app
docker image prune -f
LOG

if ! cmp -s "${TMP_DIR}/expected-reload-terminology.log" "${DEPLOY_LOG}"; then
  echo "RELOAD_TERMINOLOGY=1 should preflight and reload terminology"
  diff -u "${TMP_DIR}/expected-reload-terminology.log" "${DEPLOY_LOG}" || true
  exit 1
fi

APP_DIR="${TMP_DIR}/app-healthcheck-disabled"
DEPLOY_LOG="${TMP_DIR}/deploy-healthcheck-disabled.log"
create_app "${APP_DIR}"
cat >> "${APP_DIR}/.env.prod" <<'ENV'
HEALTHCHECK_URL=http://127.0.0.1:1/health
ENV

cat > "${APP_DIR}/run-terminology-manifest.sh" <<'SH'
#!/usr/bin/env sh
set -eu
if [ "${CHECK_ONLY:-}" != "1" ]; then
  printf 'terminology manifest\n' >> "${DEPLOY_LOG}"
fi
SH
chmod +x "${APP_DIR}/run-terminology-manifest.sh"

cat > "${BIN_DIR}/curl" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'curl %s\n' "$*" >> "${DEPLOY_LOG}"
exit 1
SH
chmod +x "${BIN_DIR}/curl"

cat > "${BIN_DIR}/sleep" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'sleep %s\n' "$*" >> "${DEPLOY_LOG}"
SH
chmod +x "${BIN_DIR}/sleep"

PATH="${BIN_DIR}:${PATH}" \
DEPLOY_LOG="${DEPLOY_LOG}" \
APP_DIR="${APP_DIR}" \
COMPOSE_FILE=docker-compose.prod.yml \
ENV_FILE=.env.prod \
IMAGE_REF=ghcr.io/example/e2br3-web-server:abc123 \
RESET_DB=0 \
HEALTHCHECK_URL="" \
sh "${SCRIPT}"

if grep -F "curl " "${DEPLOY_LOG}" >/dev/null; then
  echo "caller HEALTHCHECK_URL= should disable .env.prod healthcheck"
  cat "${DEPLOY_LOG}"
  exit 1
fi

APP_DIR="${TMP_DIR}/app-preflight-fail"
DEPLOY_LOG="${TMP_DIR}/deploy-preflight-fail.log"
create_app "${APP_DIR}"

cat > "${APP_DIR}/run-terminology-manifest.sh" <<'SH'
#!/usr/bin/env sh
set -eu
if [ "${CHECK_ONLY:-}" = "1" ]; then
  printf 'terminology preflight failed\n' >> "${DEPLOY_LOG}"
  exit 42
fi
printf 'terminology manifest\n' >> "${DEPLOY_LOG}"
SH
chmod +x "${APP_DIR}/run-terminology-manifest.sh"

if PATH="${BIN_DIR}:${PATH}" \
  DEPLOY_LOG="${DEPLOY_LOG}" \
  APP_DIR="${APP_DIR}" \
  COMPOSE_FILE=docker-compose.prod.yml \
  ENV_FILE=.env.prod \
  IMAGE_REF=ghcr.io/example/e2br3-web-server:abc123 \
  RESET_DB=1 \
  INCLUDE_SEED=1 \
  RELOAD_TERMINOLOGY=1 \
  HEALTHCHECK_URL="" \
  sh "${SCRIPT}"; then
  echo "deploy should fail when terminology preflight fails"
  exit 1
fi

grep -F "terminology preflight failed" "${DEPLOY_LOG}" >/dev/null
if grep -F "stop app" "${DEPLOY_LOG}" >/dev/null; then
  echo "deploy must not stop app when terminology preflight fails"
  exit 1
fi
if grep -F "init-rds" "${DEPLOY_LOG}" >/dev/null; then
  echo "deploy must not reset DB when terminology preflight fails"
  exit 1
fi

APP_DIR="${TMP_DIR}/app-healthcheck-rollback"
DEPLOY_LOG="${TMP_DIR}/deploy-healthcheck-rollback.log"
create_app "${APP_DIR}"
cat >> "${APP_DIR}/.env.prod" <<'ENV'
IMAGE_REF=ghcr.io/example/e2br3-web-server:old
ENV

cat > "${BIN_DIR}/curl" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'curl %s\n' "$*" >> "${DEPLOY_LOG}"
exit 1
SH
chmod +x "${BIN_DIR}/curl"

cat > "${BIN_DIR}/sleep" <<'SH'
#!/usr/bin/env sh
set -eu
printf 'sleep %s\n' "$*" >> "${DEPLOY_LOG}"
SH
chmod +x "${BIN_DIR}/sleep"

if PATH="${BIN_DIR}:${PATH}" \
  DEPLOY_LOG="${DEPLOY_LOG}" \
  APP_DIR="${APP_DIR}" \
  COMPOSE_FILE=docker-compose.prod.yml \
  ENV_FILE=.env.prod \
  IMAGE_REF=ghcr.io/example/e2br3-web-server:new \
  RESET_DB=0 \
  HEALTHCHECK_URL="http://127.0.0.1:1/health" \
  sh "${SCRIPT}"; then
  echo "deploy should fail when healthcheck fails"
  exit 1
fi

grep -Fx "IMAGE_REF=ghcr.io/example/e2br3-web-server:old" "${APP_DIR}/.env.prod" >/dev/null
grep -F "docker pull ghcr.io/example/e2br3-web-server:new" "${DEPLOY_LOG}" >/dev/null
up_count=$(grep -F "docker compose --env-file .env.prod -f docker-compose.prod.yml up -d app" "${DEPLOY_LOG}" | wc -l | tr -d ' ')
if [ "${up_count}" -ne 2 ]; then
  echo "deploy should attempt rollback by starting app with restored IMAGE_REF"
  cat "${DEPLOY_LOG}"
  exit 1
fi

echo "deploy healthcheck rollback test passed"
