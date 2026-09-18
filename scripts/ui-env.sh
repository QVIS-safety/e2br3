#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
frontend_root="${E2BR3_FRONTEND_ROOT:-$repo_root/../frontend/E2BR3-frontend}"
binary="$repo_root/target/debug/web-server"

load_cargo_env() {
	while IFS= read -r -d '' pair; do
		export "$pair"
	done < <(python3 - "$repo_root/.cargo/config.toml" <<'PY'
import os, sys, tomllib
with open(sys.argv[1], "rb") as f:
    values = tomllib.load(f).get("env", {})
for key, value in values.items():
    if key not in os.environ:
        if isinstance(value, dict):
            value = value["value"]
        sys.stdout.buffer.write(f"{key}={value}".encode() + b"\0")
PY
	)
}

database_url() {
	python3 - "$1" "$2" <<'PY'
import sys
from urllib.parse import urlsplit, urlunsplit
url, database = sys.argv[1:]
parts = urlsplit(url)
if parts.netloc:
    print(urlunsplit((parts.scheme, parts.netloc, "/" + database, parts.query, parts.fragment)))
else:
    print(f"{parts.scheme}:///{database}" + (f"?{parts.query}" if parts.query else ""))
PY
}

validate_name() {
	[[ "$1" =~ ^[a-z0-9_]+$ ]] || {
		printf 'name must contain only lowercase letters, digits, and underscores\n' >&2
		exit 2
	}
}

load_cargo_env
maintenance_url="${SERVICE_DB_URL:?SERVICE_DB_URL is required}"
admin_url="${UI_DB_ADMIN_URL:-postgres:///postgres}"

if [[ "${1:-}" == "reset" ]]; then
	name="${2:?usage: scripts/ui-env.sh reset <name>}"
	validate_name "$name"
	database_name="e2br3_ui_$name"
	dropdb --maintenance-db="$admin_url" --if-exists --force "$database_name"
	printf 'Reset %s. It will be recreated on next start.\n' "$database_name"
	exit 0
fi

name="${1:?usage: scripts/ui-env.sh <name> <backend-port> <frontend-port>}"
backend_port="${2:?backend port is required}"
frontend_port="${3:?frontend port is required}"
validate_name "$name"
[[ "$backend_port" =~ ^[0-9]+$ && "$frontend_port" =~ ^[0-9]+$ ]] || {
	printf 'ports must be numeric\n' >&2
	exit 2
}
[[ -x "$binary" ]] || {
	printf 'Missing %s; build it once with: cargo build -p web-server --bin web-server\n' "$binary" >&2
	exit 2
}
[[ -d "$frontend_root" ]] || {
	printf 'Frontend not found: %s\n' "$frontend_root" >&2
	exit 2
}

database_name="e2br3_ui_$name"
ui_url="$(database_url "$maintenance_url" "$database_name")"
admin_ui_url="$(database_url "$admin_url" "$database_name")"
app_db_user="$(python3 -c 'import sys; from urllib.parse import urlsplit; print(urlsplit(sys.argv[1]).username or "")' "$maintenance_url")"
log_dir="$repo_root/tmp/ui-env/$name"
mkdir -p "$log_dir"
ui_tsconfig="$log_dir/tsconfig.json"
printf '{"extends":"%s","compilerOptions":{"baseUrl":"%s"}}\n' "$frontend_root/tsconfig.json" "$frontend_root" >"$ui_tsconfig"
ui_tsconfig_path="$(python3 -c 'import os, sys; print(os.path.relpath(sys.argv[2], sys.argv[1]))' "$frontend_root" "$ui_tsconfig")"

if psql "$admin_url" -Atqc "SELECT 1 FROM pg_database WHERE datname = '$database_name'" | grep -q 1; then
	psql "$admin_ui_url" -Atqc "SELECT to_regclass('public.ui_env_ready')" | grep -q ui_env_ready || {
		printf 'Incomplete UI database %s; run: scripts/ui-env.sh reset %s\n' "$database_name" "$name" >&2
		exit 2
	}
elif [[ -n "${UI_DB_TEMPLATE:-}" ]]; then
	validate_name "$UI_DB_TEMPLATE"
	createdb --maintenance-db="$admin_url" --owner="$app_db_user" --template="$UI_DB_TEMPLATE" "$database_name"
	psql "$admin_ui_url" -Atqc "SELECT to_regclass('public.ui_env_ready')" | grep -q ui_env_ready || {
		printf 'Template %s is not a ready UI database.\n' "$UI_DB_TEMPLATE" >&2
		exit 2
	}
else
	createdb --maintenance-db="$admin_url" --owner="$app_db_user" "$database_name"
	: >"$log_dir/init.log"
	while IFS= read -r sql_file; do
		psql "$admin_ui_url" -v ON_ERROR_STOP=1 -v "app_db_user=$app_db_user" -f "$repo_root/db/$sql_file" >>"$log_dir/init.log" 2>&1
	done < <("$repo_root/scripts/db/list_init_sql.sh" "$repo_root/db" 1)
	psql "$admin_ui_url" -v ON_ERROR_STOP=1 -v "app_db_user=$app_db_user" <<'SQL'
GRANT USAGE ON SCHEMA public TO :"app_db_user";
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO :"app_db_user";
GRANT USAGE, SELECT, UPDATE ON ALL SEQUENCES IN SCHEMA public TO :"app_db_user";
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO :"app_db_user";
SQL
	psql "$admin_ui_url" -v ON_ERROR_STOP=1 -qc 'CREATE TABLE ui_env_ready (ready BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (ready)); INSERT INTO ui_env_ready DEFAULT VALUES;'
fi

export SERVICE_DB_URL="$ui_url"
export SERVICE_MIGRATION_DB_URL="$ui_url"
export SERVICE_BIND_ADDR="127.0.0.1:$backend_port"
export SKIP_DEV_INIT=1
export E2BR3_PUBLIC_ORIGIN="${E2BR3_PUBLIC_ORIGIN:-http://127.0.0.1:$frontend_port}"
export E2BR3_DEFAULT_MESSAGE_SENDER="${E2BR3_DEFAULT_MESSAGE_SENDER:-TEST-SENDER}"
export E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH="${E2BR3_DEFAULT_MESSAGE_RECEIVER_ICH:-TEST-ICH}"
export E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA="${E2BR3_DEFAULT_MESSAGE_RECEIVER_FDA:-TEST-FDA}"
export E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS="${E2BR3_DEFAULT_MESSAGE_RECEIVER_MFDS:-TEST-MFDS}"

(cd "$repo_root" && "$binary") >"$log_dir/backend.log" 2>&1 &
backend_pid=$!
(cd "$frontend_root" && API_PROXY_TARGET="http://127.0.0.1:$backend_port" UI_NEXT_DIST_DIR=".next-ui-$name" UI_TSCONFIG_PATH="$ui_tsconfig_path" ./node_modules/.bin/next dev --hostname 127.0.0.1 --port "$frontend_port") >"$log_dir/frontend.log" 2>&1 &
frontend_pid=$!

cleanup() {
	kill "$backend_pid" "$frontend_pid" 2>/dev/null || true
	wait "$backend_pid" "$frontend_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

printf 'UI %s: http://127.0.0.1:%s (DB %s)\n' "$name" "$frontend_port" "$database_name"
printf 'Logs: %s\n' "$log_dir"
while kill -0 "$backend_pid" 2>/dev/null && kill -0 "$frontend_pid" 2>/dev/null; do
	sleep 2
done
exit 1
