#!/usr/bin/env bash
# Isolated command-wiring fixture, not evidence of a real backend run.
set -euo pipefail
source_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/e2br3-ui-env-contract.XXXXXX")"
trap 'rm -rf "$test_root"' EXIT
repo_root="$test_root/repo"
fake_bin="$test_root/bin"
log_dir="$test_root/log"
frontend_root="$test_root/frontend"
mkdir -p "$repo_root/scripts" "$repo_root/.cargo" "$repo_root/target/debug" \
  "$repo_root/deploy/ec2" "$fake_bin" "$log_dir" "$frontend_root"
cp "$source_root/scripts/ui-env.sh" "$repo_root/scripts/ui-env.sh"
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
  *) exit 74 ;;
esac
EOF
cat >"$repo_root/deploy/ec2/migrate-rds.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >"$FAKE_LOG_DIR/baseline.args"
printf 'Frozen schema baseline mismatch\n' >&2
exit 73
EOF
cat >"$fake_bin/createdb" <<'EOF'
#!/usr/bin/env bash
printf 'Unexpected database creation\n' >&2
exit 75
EOF
chmod +x "$fake_bin/psql" "$fake_bin/createdb" "$repo_root/deploy/ec2/migrate-rds.sh"
set +e
SERVICE_DB_URL='postgres://app@db.example/app_db' \
  UI_DB_ADMIN_URL='postgres://admin@db.example/postgres' \
  E2BR3_FRONTEND_ROOT="$frontend_root" FAKE_LOG_DIR="$log_dir" PATH="$fake_bin:$PATH" \
  "$repo_root/scripts/ui-env.sh" existing 18080 13080 >"$log_dir/stdout" 2>"$log_dir/stderr"
status=$?
set -e
[[ "$status" -eq 73 ]]
[[ "$(wc -l <"$log_dir/psql.args")" -eq 2 ]]
grep -Fx -- "--db-dir $repo_root/db --check" "$log_dir/baseline.args" >/dev/null
grep -F -- 'Frozen schema baseline mismatch' "$log_dir/stderr" >/dev/null
[[ ! -e "$repo_root/tmp/ui-env/existing/backend.log" ]]
printf 'UI environment rejects mismatched baseline before startup: PASS\n'
