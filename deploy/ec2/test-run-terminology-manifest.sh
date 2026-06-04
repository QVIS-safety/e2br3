#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SCRIPT="${SCRIPT_DIR}/run-terminology-manifest.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

APP_DIR="${TMP_DIR}/app"
TERMINOLOGY_DIR="${TMP_DIR}/terminology"
INCOMING_DIR="${TERMINOLOGY_DIR}/incoming"
MANIFEST="${TERMINOLOGY_DIR}/terminology-manifest.prod"
DOCKER_LOG="${TMP_DIR}/docker.log"
mkdir -p "${APP_DIR}" "${INCOMING_DIR}" "${TMP_DIR}/bin"

cat > "${TMP_DIR}/bin/docker" <<'SH'
#!/usr/bin/env sh
set -eu
printf '%s\n' "$*" >> "${DOCKER_LOG}"
SH
chmod +x "${TMP_DIR}/bin/docker"

touch "${APP_DIR}/.env.prod" "${APP_DIR}/docker-compose.prod.yml"
touch "${INCOMING_DIR}/meddra_28_1.zip" "${INCOMING_DIR}/whodrug.zip"

cat > "${MANIFEST}" <<EOF
# production terminology files

meddra ${TMP_DIR}/terminology/incoming/meddra_28_1.zip 28.1 en
whodrug ${TMP_DIR}/terminology/incoming/whodrug.zip 2026.03 ko
EOF

PATH="${TMP_DIR}/bin:${PATH}" \
APP_DIR="${APP_DIR}" \
E2BR3_TERMINOLOGY_DIR="${TERMINOLOGY_DIR}" \
TERMINOLOGY_MANIFEST="${MANIFEST}" \
DOCKER_LOG="${DOCKER_LOG}" \
sh "${SCRIPT}"

grep -F "compose --env-file ${APP_DIR}/.env.prod -f ${APP_DIR}/docker-compose.prod.yml run --rm terminology-loader meddra --input /terminology/incoming/meddra_28_1.zip --version 28.1 --language en" "${DOCKER_LOG}" >/dev/null
grep -F "compose --env-file ${APP_DIR}/.env.prod -f ${APP_DIR}/docker-compose.prod.yml run --rm terminology-loader whodrug --input /terminology/incoming/whodrug.zip --version 2026.03 --language ko" "${DOCKER_LOG}" >/dev/null

OUTSIDE_INPUT="${TMP_DIR}/outside.zip"
OUTSIDE_MANIFEST="${TMP_DIR}/outside-manifest.prod"
touch "${OUTSIDE_INPUT}"
cat > "${OUTSIDE_MANIFEST}" <<EOF
meddra ${OUTSIDE_INPUT} 28.1 en
EOF

if PATH="${TMP_DIR}/bin:${PATH}" \
  APP_DIR="${APP_DIR}" \
  E2BR3_TERMINOLOGY_DIR="${TERMINOLOGY_DIR}" \
  TERMINOLOGY_MANIFEST="${OUTSIDE_MANIFEST}" \
  DOCKER_LOG="${DOCKER_LOG}" \
  sh "${SCRIPT}" 2>"${TMP_DIR}/outside.err"; then
  echo "script should reject input outside E2BR3_TERMINOLOGY_DIR"
  exit 1
fi
grep -F "must be under E2BR3_TERMINOLOGY_DIR" "${TMP_DIR}/outside.err" >/dev/null

echo "run-terminology-manifest tests passed"
