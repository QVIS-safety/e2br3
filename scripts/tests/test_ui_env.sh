#!/usr/bin/env bash
set -euo pipefail

source_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/e2br3-ui-env-contract.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT

repo_root="$test_root/repo"
fake_bin="$test_root/bin"
log_dir="$test_root/log"
frontend_root="$test_root/frontend"
mkdir -p "$repo_root/scripts" "$repo_root/.cargo" "$repo_root/target/debug" \
	"$repo_root/db/migrations" "$fake_bin" "$log_dir" "$frontend_root"
cp "$source_root/scripts/ui-env.sh" "$repo_root/scripts/ui-env.sh"
cp "$source_root/db/migrations/20260920_preserve_test_date_precision.sql" "$repo_root/db/migrations/"
cp "$source_root/db/migrations/20260921_fda_device_audit_triggers.sql" "$repo_root/db/migrations/"
cp "$source_root/db/migrations/20261007_drug_not_administered_terminology.sql" "$repo_root/db/migrations/"
printf '[env]\n' >"$repo_root/.cargo/config.toml"
printf '#!/usr/bin/env bash\nexit 0\n' >"$repo_root/target/debug/web-server"
chmod +x "$repo_root/target/debug/web-server"

cat >"$fake_bin/psql" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_LOG_DIR/psql.args"
case "$*" in
	*"SELECT 1 FROM pg_database"*) printf '1\n' ;;
	*"SELECT to_regclass('public.ui_env_ready')"*) printf 'ui_env_ready\n' ;;
	*"20260920_preserve_test_date_precision.sql"*) exit 0 ;;
	*"20260921_fda_device_audit_triggers.sql"*) exit 0 ;;
	*"20261007_drug_not_administered_terminology.sql"*)
		printf 'fixture migration failure\n' >&2
		exit 73
		;;
	*)
		printf 'unexpected psql call: %s\n' "$*" >&2
		exit 74
		;;
esac
EOF
cat >"$fake_bin/createdb" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_LOG_DIR/createdb.args"
exit 75
EOF
chmod +x "$fake_bin/psql" "$fake_bin/createdb"

set +e
SERVICE_DB_URL='postgres://app@db.example/app_db' \
	UI_DB_ADMIN_URL='postgres://admin@db.example/postgres' \
	E2BR3_FRONTEND_ROOT="$frontend_root" \
	FAKE_LOG_DIR="$log_dir" \
	PATH="$fake_bin:$PATH" \
	"$repo_root/scripts/ui-env.sh" existing 18080 13080 \
	>"$log_dir/stdout" 2>"$log_dir/stderr"
status=$?
set -e

[[ "$status" -eq 2 ]]
[[ ! -e "$log_dir/createdb.args" ]]
[[ "$(wc -l <"$log_dir/psql.args")" -eq 5 ]]
grep -F -- '-v ON_ERROR_STOP=1 -f' "$log_dir/psql.args" >/dev/null
grep -F -- "$repo_root/db/migrations/20260920_preserve_test_date_precision.sql" "$log_dir/psql.args" >/dev/null
grep -F -- "$repo_root/db/migrations/20260921_fda_device_audit_triggers.sql" "$log_dir/psql.args" >/dev/null
grep -F -- "$repo_root/db/migrations/20261007_drug_not_administered_terminology.sql" "$log_dir/psql.args" >/dev/null
grep -F -- 'fixture migration failure' "$log_dir/stderr" >/dev/null
grep -F -- 'Failed to upgrade UI database e2br3_ui_existing with db/migrations/20261007_drug_not_administered_terminology.sql.' "$log_dir/stderr" >/dev/null
printf 'UI environment existing-database upgrade contract: PASS\n'
