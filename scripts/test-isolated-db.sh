#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
maintenance_url="${SERVICE_DB_URL:?SERVICE_DB_URL is required}"

needs_xml_assets="$(
	python3 "$repo_root/scripts/ci_test_plan.py" xml-assets -- "$@"
)"
case "$needs_xml_assets" in
	true)
		"$repo_root/scripts/prepare-xml-assets.sh"
		;;
	false) ;;
	*)
		printf 'invalid XML asset selection: %s\n' "$needs_xml_assets" >&2
		exit 2
		;;
esac

for required_command in createdb psql dropdb cargo python3; do
	if ! command -v "$required_command" >/dev/null 2>&1; then
		printf 'required command not found: %s\n' "$required_command" >&2
		exit 127
	fi
done

app_db_user="$(python3 -c 'import sys; from urllib.parse import urlsplit; print(urlsplit(sys.argv[1]).username or "")' "${maintenance_url}")"
if [[ -z "$app_db_user" ]]; then
	printf 'SERVICE_DB_URL must include a database user\n' >&2
	exit 2
fi

database_name="e2br3_test_$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')"
if [[ ! "$database_name" =~ ^e2br3_test_[a-z0-9_]+$ ]]; then
	printf 'refusing unsafe test database name\n' >&2
	exit 2
fi

url_without_query="${maintenance_url%%\?*}"
if [[ "$url_without_query" != *://*/* ]]; then
	printf 'SERVICE_DB_URL must include a database path\n' >&2
	exit 2
fi
query_suffix=""
if [[ "$maintenance_url" == *\?* ]]; then
	query_suffix="?${maintenance_url#*\?}"
fi
test_url="${url_without_query%/*}/${database_name}${query_suffix}"
created=0
active_child_pid=""
active_child_signalable=0
interrupted_signal=""
interrupted_status=0

forward_signal() {
	if [[ "$interrupted_status" -eq 0 ]]; then
		interrupted_signal="$1"
		interrupted_status="$2"
	fi
	if [[ "$active_child_signalable" -eq 1 ]] && \
		[[ -n "$active_child_pid" ]] && \
		kill -0 "$active_child_pid" 2>/dev/null; then
		kill "-$interrupted_signal" -- "-$active_child_pid" 2>/dev/null || true
	fi
}

run_interruptible() {
	if [[ "$interrupted_status" -ne 0 ]]; then
		return "$interrupted_status"
	fi

	active_child_signalable=1
	set -m
	"$@" &
	active_child_pid=$!
	set +m

	if [[ -n "$interrupted_signal" ]]; then
		kill "-$interrupted_signal" -- "-$active_child_pid" 2>/dev/null || true
	fi

	set +e
	wait "$active_child_pid"
	local child_status=$?
	if [[ "$interrupted_status" -ne 0 ]]; then
		while kill -0 "$active_child_pid" 2>/dev/null; do
			wait "$active_child_pid" 2>/dev/null
		done
		child_status="$interrupted_status"
	fi
	set -e
	active_child_pid=""
	active_child_signalable=0
	return "$child_status"
}

create_database() {
	if [[ "$interrupted_status" -ne 0 ]]; then
		return "$interrupted_status"
	fi

	active_child_signalable=0
	set -m
	createdb --maintenance-db="$maintenance_url" "$database_name" &
	active_child_pid=$!
	set +m

	set +e
	wait "$active_child_pid"
	local child_status=$?
	if [[ "$interrupted_status" -ne 0 ]]; then
		while kill -0 "$active_child_pid" 2>/dev/null; do
			wait "$active_child_pid" 2>/dev/null
		done
		wait "$active_child_pid" 2>/dev/null
		child_status=$?
	fi
	set -e
	active_child_pid=""

	if [[ "$child_status" -eq 0 ]]; then
		created=1
	fi
	if [[ "$interrupted_status" -ne 0 ]]; then
		return "$interrupted_status"
	fi
	return "$child_status"
}

cleanup() {
	local status=$?
	trap - EXIT INT TERM
	if [[ "$created" -eq 1 ]]; then
		if [[ ! "$database_name" =~ ^e2br3_test_[a-z0-9_]+$ ]]; then
			printf 'refusing unsafe test database cleanup\n' >&2
			if [[ "$status" -eq 0 ]]; then
				status=1
			fi
		else
			printf 'Dropping isolated test database %s\n' "$database_name"
			if ! dropdb --maintenance-db="$maintenance_url" --force "$database_name"; then
				printf 'failed to drop isolated test database %s\n' "$database_name" >&2
				if [[ "$status" -eq 0 ]]; then
					status=1
				fi
			fi
		fi
	fi
	exit "$status"
}

trap cleanup EXIT
trap 'forward_signal INT 130' INT
trap 'forward_signal TERM 143' TERM

printf 'Creating isolated test database %s\n' "$database_name"
create_database

sql_list="$("$repo_root/scripts/db/list_init_sql.sh" "$repo_root/db" 1)"
while IFS= read -r sql_file; do
	if [[ -n "$sql_file" ]]; then
		run_interruptible \
			psql "$test_url" -v ON_ERROR_STOP=1 -v "app_db_user=${app_db_user}" \
				-f "$repo_root/db/$sql_file"
	fi
done <<<"$sql_list"

export SERVICE_DB_URL="$test_url"
export SERVICE_MIGRATION_DB_URL="$test_url"
export SKIP_DEV_INIT=1
export E2BR3_TEST_DATABASE_NAME="$database_name"

run_interruptible cargo test "$@"
