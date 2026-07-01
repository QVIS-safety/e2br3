#!/usr/bin/env sh
set -eu

APP_DIR="${APP_DIR:-/opt/e2br3}"
ENV_FILE="${ENV_FILE:-.env.prod}"
SOURCE_URL="${SOURCE_URL:-https://datahub.io/core/country-list/_r/-/data.csv}"

usage() {
  cat >&2 <<'EOF'
Usage:
  deploy/ec2/load-iso-countries.sh

Environment:
  APP_DIR       EC2 app directory. Default: /opt/e2br3
  ENV_FILE      Env file, relative to APP_DIR unless absolute. Default: .env.prod
  SERVICE_DB_URL  Optional DB URL override. If unset, loaded from ENV_FILE.
  SOURCE_URL    Optional ISO country CSV URL override.

The CSV must have DataHub/Core country-list columns: Name,Code.
EOF
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
  "")
    ;;
  *)
    usage
    exit 2
    ;;
esac

case "${ENV_FILE}" in
  /*) RESOLVED_ENV_FILE="${ENV_FILE}" ;;
  *) RESOLVED_ENV_FILE="${APP_DIR}/${ENV_FILE}" ;;
esac

if [ ! -f "${RESOLVED_ENV_FILE}" ]; then
  echo "Missing env file: ${RESOLVED_ENV_FILE}" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required." >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required. Install postgresql-client on the EC2 host." >&2
  exit 1
fi

if [ -z "${SERVICE_DB_URL:-}" ]; then
  SERVICE_DB_URL=$(
    sed -n 's/^[[:space:]]*SERVICE_DB_URL[[:space:]]*=[[:space:]]*//p' "${RESOLVED_ENV_FILE}" \
      | tail -n 1 \
      | sed 's/^"//; s/"$//; s/^'\''//; s/'\''$//'
  )
fi

if [ -z "${SERVICE_DB_URL:-}" ]; then
  echo "SERVICE_DB_URL is required in environment or ${RESOLVED_ENV_FILE}." >&2
  exit 1
fi

TMP_DIR=$(mktemp -d)
CSV_FILE="${TMP_DIR}/iso-countries.csv"
trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

echo "Downloading ISO country CSV: ${SOURCE_URL}"
curl -fsSL -o "${CSV_FILE}" "${SOURCE_URL}"

echo "Loading ISO countries into iso_countries..."
psql "${SERVICE_DB_URL}" -v ON_ERROR_STOP=1 -v csv_file="${CSV_FILE}" -v source_url="${SOURCE_URL}" <<'SQL'
BEGIN;

SELECT set_current_user_context('00000000-0000-0000-0000-000000000001'::uuid);
SELECT set_org_context('00000000-0000-0000-0000-000000000000'::uuid, 'system_admin');

CREATE TEMP TABLE staging_iso_countries (
  name text NOT NULL,
  code text NOT NULL
) ON COMMIT DROP;

\copy staging_iso_countries(name, code) FROM :'csv_file' WITH (FORMAT csv, HEADER true)

WITH normalized AS (
  SELECT DISTINCT ON (upper(trim(code)))
    upper(trim(code)) AS code,
    nullif(trim(name), '') AS name
  FROM staging_iso_countries
  WHERE trim(code) ~ '^[A-Za-z]{2}$'
    AND nullif(trim(name), '') IS NOT NULL
  ORDER BY upper(trim(code)), trim(name)
),
upserted AS (
  INSERT INTO iso_countries (code, name, active)
  SELECT code, name, true
  FROM normalized
  ON CONFLICT (code)
  DO UPDATE SET
    name = EXCLUDED.name,
    active = true
  RETURNING code
)
UPDATE iso_countries AS country
SET active = false
WHERE country.active = true
  AND NOT EXISTS (
    SELECT 1
    FROM normalized
    WHERE normalized.code = country.code
  );

DO $$
DECLARE
  loaded_count integer;
BEGIN
  SELECT count(*) INTO loaded_count
  FROM iso_countries
  WHERE active = true;

  IF loaded_count < 200 THEN
    RAISE EXCEPTION 'ISO country load produced too few active rows: %', loaded_count;
  END IF;

  RAISE NOTICE 'Loaded active ISO countries: %', loaded_count;
END $$;

COMMIT;

\echo Loaded ISO countries from :source_url
SQL

echo "ISO country load complete."
