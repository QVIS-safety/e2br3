#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
runner="$repo_root/scripts/test-isolated-db.sh"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/e2br3-isolated-contract.XXXXXX")"
fake_bin="$test_root/bin"
mkdir -p "$fake_bin"

cleanup_test_root() {
	rm -rf "$test_root"
}
trap cleanup_test_root EXIT

write_fake_commands() {
cat >"$fake_bin/createdb" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_LOG_DIR/createdb.args"
if [[ "${FAKE_CREATEDB_WAIT:-0}" -eq 1 ]]; then
	printf '%s\n' "$$" >"$FAKE_LOG_DIR/createdb.pid"
	trap 'printf "TERM\n" >"$FAKE_LOG_DIR/createdb.signal"; exit 143' TERM
	sleep 1
	printf 'completed\n' >"$FAKE_LOG_DIR/createdb.completed"
fi
EOF
cat >"$fake_bin/psql" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_LOG_DIR/psql.args"
cat >"$FAKE_LOG_DIR/psql.stdin"
if [[ "${FAKE_PSQL_WAIT:-0}" -eq 1 ]]; then
	printf '%s\n' "$$" >"$FAKE_LOG_DIR/psql.pid"
	trap 'printf "TERM\n" >"$FAKE_LOG_DIR/psql.signal"; exit 143' TERM
	while true; do
		sleep 1
	done
fi
exit "${FAKE_PSQL_EXIT:-0}"
EOF
	cat >"$fake_bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >"$FAKE_LOG_DIR/cargo.args"
{
	printf 'SERVICE_DB_URL=%s\n' "$SERVICE_DB_URL"
	printf 'SERVICE_MIGRATION_DB_URL=%s\n' "$SERVICE_MIGRATION_DB_URL"
	printf 'SKIP_DEV_INIT=%s\n' "$SKIP_DEV_INIT"
	printf 'E2BR3_TEST_DATABASE_NAME=%s\n' "$E2BR3_TEST_DATABASE_NAME"
} >"$FAKE_LOG_DIR/cargo.env"
if [[ "${FAKE_CARGO_WAIT:-0}" -eq 1 ]]; then
	printf '%s\n' "$$" >"$FAKE_LOG_DIR/cargo.pid"
	trap 'printf "INT\n" >"$FAKE_LOG_DIR/cargo.signal"; exit 130' INT
	trap 'printf "TERM\n" >"$FAKE_LOG_DIR/cargo.signal"; exit 143' TERM
	while true; do
		sleep 1
	done
fi
exit "${FAKE_CARGO_EXIT:-0}"
EOF
	cat >"$fake_bin/dropdb" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_LOG_DIR/dropdb.args"
exit "${FAKE_DROPDB_EXIT:-0}"
EOF
	chmod +x "$fake_bin/createdb" "$fake_bin/psql" "$fake_bin/cargo" "$fake_bin/dropdb"
}

new_case_dir() {
	local name="$1"
	local case_dir="$test_root/$name"
	mkdir -p "$case_dir"
	printf '%s\n' "$case_dir"
}

assert_success_contract() {
	local case_dir
	case_dir="$(new_case_dir success)"
	SERVICE_DB_URL='postgres://user:secret@db:5432/app_db?sslmode=disable' \
		FAKE_LOG_DIR="$case_dir" \
		PATH="$fake_bin:$PATH" \
		"$runner" -p lib-core --test rbac_grant_profiles \
		>"$case_dir/stdout" 2>"$case_dir/stderr"

	local test_url migration_url database_name
	test_url="$(sed -n 's/^SERVICE_DB_URL=//p' "$case_dir/cargo.env")"
	migration_url="$(sed -n 's/^SERVICE_MIGRATION_DB_URL=//p' "$case_dir/cargo.env")"
	database_name="$(sed -n 's/^E2BR3_TEST_DATABASE_NAME=//p' "$case_dir/cargo.env")"

	[[ "$database_name" =~ ^e2br3_test_[a-z0-9_]+$ ]]
	[[ "$test_url" == "postgres://user:secret@db:5432/$database_name?sslmode=disable" ]]
	[[ "$migration_url" == "$test_url" ]]
	grep -Fx -- 'test -p lib-core --test rbac_grant_profiles' "$case_dir/cargo.args" >/dev/null
	grep -Fx -- 'SKIP_DEV_INIT=1' "$case_dir/cargo.env" >/dev/null
	grep -F -- "$database_name" "$case_dir/createdb.args" >/dev/null
	grep -F -- "$database_name" "$case_dir/dropdb.args" >/dev/null

	grep -Fx -- "-X -w -d $test_url -v ON_ERROR_STOP=1 -f - -v app_db_user=user" "$case_dir/psql.args" >/dev/null
	grep -F -- 'CREATE TABLE public.schema_migrations' "$case_dir/psql.stdin" >/dev/null
	grep -F -- 'pg_try_advisory_lock' "$case_dir/psql.stdin" >/dev/null

	if grep -F -- 'user:secret' "$case_dir/stdout" "$case_dir/stderr" >/dev/null; then
		printf 'runner leaked database credentials\n' >&2
		return 1
	fi
}

assert_bootstrap_role_wiring() {
	grep -F -- 'migrate-rds.py' "$repo_root/deploy/docker/postgres-init/01-run-all.sh" >/dev/null
	grep -F -- 'app_db_user=' "$repo_root/deploy/ec2/migrate-rds.py" >/dev/null
	grep -F -- 'GRANT e2br3_app_role TO :"app_db_user";' \
		"$repo_root/db/bootstrap/01-safetydb-schema.sql" >/dev/null
	grep -F -- 'GRANT e2br3_auditor_role TO :"app_db_user";' \
		"$repo_root/db/bootstrap/01-safetydb-schema.sql" >/dev/null
}

assert_interrupt_forwards_signal_and_cleans_up() {
	local signal_name="$1"
	local expected_status="$2"
	local case_dir
	case_dir="$(new_case_dir "interrupt-${signal_name}")"
	set -m
	SERVICE_DB_URL='postgres://user:secret@db:5432/app_db?sslmode=disable' \
		FAKE_LOG_DIR="$case_dir" \
		FAKE_CARGO_WAIT=1 \
		PATH="$fake_bin:$PATH" \
		"$runner" -p lib-core >"$case_dir/stdout" 2>"$case_dir/stderr" &
	local runner_pid=$!
	set +m

	local attempt
	for attempt in {1..100}; do
		if [[ -s "$case_dir/cargo.pid" ]]; then
			break
		fi
		sleep 0.05
	done
	test -s "$case_dir/cargo.pid"
	local cargo_pid
	cargo_pid="$(cat "$case_dir/cargo.pid")"

	kill "-$signal_name" "$runner_pid"
	local runner_finished=0
	for attempt in {1..40}; do
		if ! kill -0 "$runner_pid" 2>/dev/null; then
			runner_finished=1
			break
		fi
		sleep 0.05
	done

	if [[ "$runner_finished" -eq 0 ]]; then
		kill -TERM "$cargo_pid" 2>/dev/null || true
	fi
	set +e
	wait "$runner_pid"
	local status=$?
	set -e

	if [[ "$runner_finished" -ne 1 ]]; then
		printf 'runner did not exit after forwarding %s to cargo\n' "$signal_name" >&2
		return 1
	fi
	[[ "$status" -eq "$expected_status" ]]
	grep -Fx -- "$signal_name" "$case_dir/cargo.signal" >/dev/null
	test -s "$case_dir/dropdb.args"
	if kill -0 "$cargo_pid" 2>/dev/null; then
		printf 'runner left cargo process alive after TERM\n' >&2
		return 1
	fi
}

assert_interrupt_stops_database_initialization() {
	local case_dir
	case_dir="$(new_case_dir interrupt-initialization)"
	set -m
	SERVICE_DB_URL='postgres://user:secret@db:5432/app_db?sslmode=disable' \
		FAKE_LOG_DIR="$case_dir" \
		FAKE_PSQL_WAIT=1 \
		PATH="$fake_bin:$PATH" \
		"$runner" -p lib-core >"$case_dir/stdout" 2>"$case_dir/stderr" &
	local runner_pid=$!
	set +m

	local attempt
	for attempt in {1..100}; do
		if [[ -s "$case_dir/psql.pid" ]]; then
			break
		fi
		sleep 0.05
	done
	test -s "$case_dir/psql.pid"
	local psql_pid
	psql_pid="$(cat "$case_dir/psql.pid")"

	kill -TERM "$runner_pid"
	local runner_finished=0
	for attempt in {1..40}; do
		if ! kill -0 "$runner_pid" 2>/dev/null; then
			runner_finished=1
			break
		fi
		sleep 0.05
	done

	if [[ "$runner_finished" -eq 0 ]]; then
		kill -TERM "$psql_pid" 2>/dev/null || true
	fi
	set +e
	wait "$runner_pid"
	local status=$?
	set -e

	if [[ "$runner_finished" -ne 1 ]]; then
		printf 'runner did not stop active database initialization on TERM\n' >&2
		return 1
	fi
	[[ "$status" -eq 143 ]]
	grep -Fx -- 'TERM' "$case_dir/psql.signal" >/dev/null
	test -s "$case_dir/dropdb.args"
	test ! -e "$case_dir/cargo.args"
}

assert_interrupt_waits_for_database_creation_then_cleans_up() {
	local case_dir
	case_dir="$(new_case_dir interrupt-creation)"
	set -m
	SERVICE_DB_URL='postgres://user:secret@db:5432/app_db?sslmode=disable' \
		FAKE_LOG_DIR="$case_dir" \
		FAKE_CREATEDB_WAIT=1 \
		PATH="$fake_bin:$PATH" \
		"$runner" -p lib-core >"$case_dir/stdout" 2>"$case_dir/stderr" &
	local runner_pid=$!
	set +m

	local attempt
	for attempt in {1..100}; do
		if [[ -s "$case_dir/createdb.pid" ]]; then
			break
		fi
		sleep 0.05
	done
	test -s "$case_dir/createdb.pid"

	kill -TERM "$runner_pid"
	set +e
	wait "$runner_pid"
	local status=$?
	set -e

	[[ "$status" -eq 143 ]]
	if [[ ! -s "$case_dir/createdb.completed" ]]; then
		printf 'runner terminated createdb before ownership was known\n' >&2
		return 1
	fi
	test ! -e "$case_dir/createdb.signal"
	test -s "$case_dir/dropdb.args"
	test ! -e "$case_dir/psql.args"
	test ! -e "$case_dir/cargo.args"
}

assert_test_failure_preserves_status_and_cleans_up() {
	local case_dir
	case_dir="$(new_case_dir cargo-failure)"
	set +e
	SERVICE_DB_URL='postgres://user:secret@db:5432/app_db?sslmode=disable' \
		FAKE_LOG_DIR="$case_dir" \
		FAKE_CARGO_EXIT=23 \
		PATH="$fake_bin:$PATH" \
		"$runner" -p lib-core >"$case_dir/stdout" 2>"$case_dir/stderr"
	local status=$?
	set -e
	[[ "$status" -eq 23 ]]
	test -s "$case_dir/dropdb.args"
}

assert_cleanup_failure_fails_successful_run() {
	local case_dir
	case_dir="$(new_case_dir cleanup-failure)"
	set +e
	SERVICE_DB_URL='postgres://user:secret@db:5432/app_db?sslmode=disable' \
		FAKE_LOG_DIR="$case_dir" \
		FAKE_DROPDB_EXIT=9 \
		PATH="$fake_bin:$PATH" \
		"$runner" -p lib-core >"$case_dir/stdout" 2>"$case_dir/stderr"
	local status=$?
	set -e
	[[ "$status" -ne 0 ]]
	test -s "$case_dir/dropdb.args"
}

assert_initialization_failure_preserves_status_and_cleans_up() {
	local case_dir
	case_dir="$(new_case_dir psql-failure)"
	set +e
	SERVICE_DB_URL='postgres://user:secret@db:5432/app_db?sslmode=disable' \
		FAKE_LOG_DIR="$case_dir" \
		FAKE_PSQL_EXIT=31 \
		PATH="$fake_bin:$PATH" \
		"$runner" -p lib-core >"$case_dir/stdout" 2>"$case_dir/stderr"
	local status=$?
	set -e
	[[ "$status" -eq 31 ]]
	test -s "$case_dir/dropdb.args"
}

assert_repository_wiring() {
	grep -F -- 'scripts/test-isolated-db.sh' "$repo_root/scripts/ci_test_plan.py" >/dev/null
	grep -F -- 'from ci_test_plan import run_commands' "$repo_root/.github/workflows/ci.yml" >/dev/null
}

write_fake_commands
assert_success_contract
assert_interrupt_forwards_signal_and_cleans_up TERM 143
assert_interrupt_forwards_signal_and_cleans_up INT 130
assert_interrupt_waits_for_database_creation_then_cleans_up
assert_interrupt_stops_database_initialization
assert_test_failure_preserves_status_and_cleans_up
assert_cleanup_failure_fails_successful_run
assert_initialization_failure_preserves_status_and_cleans_up
assert_bootstrap_role_wiring
assert_repository_wiring
printf 'isolated database runner contract: PASS\n'
