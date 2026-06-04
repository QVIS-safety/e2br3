#!/usr/bin/env sh
set -eu

APP_DIR=${APP_DIR:-/opt/e2br3}
ENV_FILE=${ENV_FILE:-.env.prod}
COMPOSE_FILE=${COMPOSE_FILE:-docker-compose.prod.yml}
E2BR3_TERMINOLOGY_DIR=${E2BR3_TERMINOLOGY_DIR:-/opt/e2br3/terminology}
TERMINOLOGY_MANIFEST=${TERMINOLOGY_MANIFEST:-${E2BR3_TERMINOLOGY_DIR}/terminology-manifest.prod}

case "${ENV_FILE}" in
  /*) ENV_FILE_PATH=${ENV_FILE} ;;
  *) ENV_FILE_PATH="${APP_DIR}/${ENV_FILE}" ;;
esac

case "${COMPOSE_FILE}" in
  /*) COMPOSE_FILE_PATH=${COMPOSE_FILE} ;;
  *) COMPOSE_FILE_PATH="${APP_DIR}/${COMPOSE_FILE}" ;;
esac

if [ ! -f "${ENV_FILE_PATH}" ]; then
  echo "Missing env file: ${ENV_FILE_PATH}" >&2
  exit 1
fi

if [ ! -f "${COMPOSE_FILE_PATH}" ]; then
  echo "Missing compose file: ${COMPOSE_FILE_PATH}" >&2
  exit 1
fi

if [ ! -f "${TERMINOLOGY_MANIFEST}" ]; then
  echo "Missing terminology manifest: ${TERMINOLOGY_MANIFEST}" >&2
  exit 1
fi

loaded=0
cd "${APP_DIR}"

while IFS= read -r line || [ -n "${line}" ]; do
  case "${line}" in
    ''|'#'*) continue ;;
  esac

  set -f
  set -- ${line}
  set +f

  if [ "$#" -gt 4 ]; then
    echo "Invalid terminology manifest line, expected 3 or 4 fields: ${line}" >&2
    exit 1
  fi

  dictionary=${1:-}
  input_path=${2:-}
  version=${3:-}
  language=${4:-en}

  if [ -z "${dictionary}" ] || [ -z "${input_path}" ] || [ -z "${version}" ]; then
    echo "Invalid manifest line: ${line}" >&2
    exit 1
  fi

  case "${dictionary}" in
    meddra|whodrug) ;;
    *)
      echo "Unsupported dictionary: ${dictionary}" >&2
      exit 1
      ;;
  esac

  if [ ! -e "${input_path}" ]; then
    echo "Missing terminology input: ${input_path}" >&2
    exit 1
  fi

  case "${input_path}" in
    "${E2BR3_TERMINOLOGY_DIR}"/*)
      relative_input=${input_path#"${E2BR3_TERMINOLOGY_DIR}/"}
      container_input="/terminology/${relative_input}"
      ;;
    *)
      echo "Terminology input must be under E2BR3_TERMINOLOGY_DIR: ${input_path}" >&2
      exit 1
      ;;
  esac

  if [ "${CHECK_ONLY:-}" != "1" ]; then
    docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" run --rm terminology-loader \
      "${dictionary}" \
      --input "${container_input}" \
      --version "${version}" \
      --language "${language}"
  fi
  loaded=$((loaded + 1))
done < "${TERMINOLOGY_MANIFEST}"

if [ "${loaded}" -eq 0 ]; then
  echo "Terminology manifest contained no loadable entries: ${TERMINOLOGY_MANIFEST}" >&2
  exit 1
fi

if [ "${CHECK_ONLY:-}" = "1" ]; then
  echo "Terminology manifest check complete: entries=${loaded}"
else
  echo "Terminology manifest complete: loaded=${loaded}"
fi
