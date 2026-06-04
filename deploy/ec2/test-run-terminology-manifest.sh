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
printf 'CALL\n' >> "${DOCKER_LOG}"
i=0
for arg do
  i=$((i + 1))
  printf 'ARG%s=%s\n' "$i" "$arg" >> "${DOCKER_LOG}"
done
SH
chmod +x "${TMP_DIR}/bin/docker"

touch "${APP_DIR}/.env.prod" "${APP_DIR}/docker-compose.prod.yml"
touch "${INCOMING_DIR}/meddra_28_1.zip" "${INCOMING_DIR}/whodrug.zip"

if PATH="${TMP_DIR}/bin:${PATH}" \
  APP_DIR="${APP_DIR}" \
  sh "${SCRIPT}" 2>"${TMP_DIR}/default-manifest.err"; then
  echo "script should use the production terminology directory default"
  exit 1
fi
grep -F "Missing terminology manifest: /opt/e2br3/terminology/terminology-manifest.prod" "${TMP_DIR}/default-manifest.err" >/dev/null

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

awk '
  /^CALL$/ { call += 1; next }
  call == 1 && $0 == "ARG1=compose" { first_arg1 = 1 }
  call == 1 && $0 == "ARG2=--env-file" { first_arg2 = 1 }
  call == 1 && $0 == "ARG3=.env.prod" { first_arg3 = 1 }
  call == 1 && $0 == "ARG4=-f" { first_arg4 = 1 }
  call == 1 && $0 == "ARG5=docker-compose.prod.yml" { first_arg5 = 1 }
  call == 1 && $0 == "ARG6=run" { first_arg6 = 1 }
  call == 1 && $0 == "ARG7=--rm" { first_arg7 = 1 }
  call == 1 && $0 == "ARG8=terminology-loader" { first_arg8 = 1 }
  call == 1 && $0 == "ARG9=meddra" { first_arg9 = 1 }
  call == 1 && $0 == "ARG10=--input" { first_arg10 = 1 }
  call == 1 && $0 == "ARG11=/terminology/incoming/meddra_28_1.zip" { first_arg11 = 1 }
  call == 1 && $0 == "ARG12=--version" { first_arg12 = 1 }
  call == 1 && $0 == "ARG13=28.1" { first_arg13 = 1 }
  call == 1 && $0 == "ARG14=--language" { first_arg14 = 1 }
  call == 1 && $0 == "ARG15=en" { first_arg15 = 1 }
  call == 2 && $0 == "ARG1=compose" { second_arg1 = 1 }
  call == 2 && $0 == "ARG2=--env-file" { second_arg2 = 1 }
  call == 2 && $0 == "ARG3=.env.prod" { second_arg3 = 1 }
  call == 2 && $0 == "ARG4=-f" { second_arg4 = 1 }
  call == 2 && $0 == "ARG5=docker-compose.prod.yml" { second_arg5 = 1 }
  call == 2 && $0 == "ARG6=run" { second_arg6 = 1 }
  call == 2 && $0 == "ARG7=--rm" { second_arg7 = 1 }
  call == 2 && $0 == "ARG8=terminology-loader" { second_arg8 = 1 }
  call == 2 && $0 == "ARG9=whodrug" { second_arg9 = 1 }
  call == 2 && $0 == "ARG10=--input" { second_arg10 = 1 }
  call == 2 && $0 == "ARG11=/terminology/incoming/whodrug.zip" { second_arg11 = 1 }
  call == 2 && $0 == "ARG12=--version" { second_arg12 = 1 }
  call == 2 && $0 == "ARG13=2026.03" { second_arg13 = 1 }
  call == 2 && $0 == "ARG14=--language" { second_arg14 = 1 }
  call == 2 && $0 == "ARG15=ko" { second_arg15 = 1 }
  END {
    if (call != 2 ||
      !first_arg1 || !first_arg2 || !first_arg3 || !first_arg4 || !first_arg5 ||
      !first_arg6 || !first_arg7 || !first_arg8 || !first_arg9 || !first_arg10 ||
      !first_arg11 || !first_arg12 || !first_arg13 || !first_arg14 || !first_arg15 ||
      !second_arg1 || !second_arg2 || !second_arg3 || !second_arg4 || !second_arg5 ||
      !second_arg6 || !second_arg7 || !second_arg8 || !second_arg9 || !second_arg10 ||
      !second_arg11 || !second_arg12 || !second_arg13 || !second_arg14 || !second_arg15) {
      exit 1
    }
  }
' "${DOCKER_LOG}"

rm -f "${DOCKER_LOG}"
PATH="${TMP_DIR}/bin:${PATH}" \
APP_DIR="${APP_DIR}" \
E2BR3_TERMINOLOGY_DIR="${TERMINOLOGY_DIR}" \
TERMINOLOGY_MANIFEST="${MANIFEST}" \
DOCKER_LOG="${DOCKER_LOG}" \
CHECK_ONLY=1 \
sh "${SCRIPT}" >"${TMP_DIR}/check-only.out"

grep -F "Terminology manifest check complete: entries=2" "${TMP_DIR}/check-only.out" >/dev/null
if [ -e "${DOCKER_LOG}" ] && [ -s "${DOCKER_LOG}" ]; then
  echo "CHECK_ONLY=1 must not invoke docker compose"
  exit 1
fi

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

WILDCARD_MANIFEST="${TMP_DIR}/wildcard-manifest.prod"
touch "${INCOMING_DIR}/wildcard-a.zip" "${INCOMING_DIR}/wildcard-b.zip"
cat > "${WILDCARD_MANIFEST}" <<EOF
meddra ${INCOMING_DIR}/wildcard-*.zip 28.1 en
EOF

if PATH="${TMP_DIR}/bin:${PATH}" \
  APP_DIR="${APP_DIR}" \
  E2BR3_TERMINOLOGY_DIR="${TERMINOLOGY_DIR}" \
  TERMINOLOGY_MANIFEST="${WILDCARD_MANIFEST}" \
  DOCKER_LOG="${DOCKER_LOG}" \
  sh "${SCRIPT}" 2>"${TMP_DIR}/wildcard.err"; then
  echo "script should reject wildcard input instead of expanding fields"
  exit 1
fi
grep -F "Missing terminology input: ${INCOMING_DIR}/wildcard-*.zip" "${TMP_DIR}/wildcard.err" >/dev/null

EXTRA_FIELDS_MANIFEST="${TMP_DIR}/extra-fields-manifest.prod"
cat > "${EXTRA_FIELDS_MANIFEST}" <<EOF
meddra ${INCOMING_DIR}/meddra_28_1.zip 28.1 en extra
EOF

if PATH="${TMP_DIR}/bin:${PATH}" \
  APP_DIR="${APP_DIR}" \
  E2BR3_TERMINOLOGY_DIR="${TERMINOLOGY_DIR}" \
  TERMINOLOGY_MANIFEST="${EXTRA_FIELDS_MANIFEST}" \
  DOCKER_LOG="${DOCKER_LOG}" \
  sh "${SCRIPT}" 2>"${TMP_DIR}/extra-fields.err"; then
  echo "script should reject manifest lines with more than four fields"
  exit 1
fi
grep -F "Invalid terminology manifest line, expected 3 or 4 fields" "${TMP_DIR}/extra-fields.err" >/dev/null

echo "run-terminology-manifest tests passed"
