#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BOOTSTRAP_SQL="${SCRIPT_DIR}/10-triggers.sql"

awk '
  /CREATE POLICY iso_countries_read ON iso_countries/ { in_policy = 1 }
  in_policy { policy = policy $0 "\n" }
  in_policy && /;/ { print policy; exit }
' "${BOOTSTRAP_SQL}" | grep -F "USING (active = true OR is_current_user_admin());" >/dev/null

echo "terminology RLS policy tests passed"
