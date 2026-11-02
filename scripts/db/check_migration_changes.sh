#!/usr/bin/env sh
set -eu

BASE_REF="${1:-${GITHUB_EVENT_BEFORE:-}}"
HEAD_REF="${2:-${GITHUB_SHA:-HEAD}}"
ZERO_SHA=0000000000000000000000000000000000000000

if [ -z "${BASE_REF}" ] || [ "${BASE_REF}" = "${ZERO_SHA}" ]; then
	BASE_REF=$(git rev-parse "${HEAD_REF}^")
fi

added_migration=0
baseline_change=0
errors=
changes=$(git diff --name-status --no-renames "${BASE_REF}" "${HEAD_REF}" -- db)
base_migrations=$(git ls-tree -r --name-only "${BASE_REF}" -- db/migrations)
last_migration=$(printf '%s\n' "$base_migrations" | sed -n '\|^db/migrations/[^/]*\.sql$|p' | LC_ALL=C sort | tail -n 1)

while IFS="$(printf '\t')" read -r status path; do
	[ -n "${path}" ] || continue

	case "${path}" in
		db/migrations/*.sql)
			if [ "${status}" = "A" ]; then
				added_migration=1
				if [ -n "$last_migration" ] && \
					[ "$(printf '%s\n%s\n' "$path" "$last_migration" | LC_ALL=C sort | head -n 1)" = "$path" ]; then
					errors="${errors}\nNew migrations must sort after ${last_migration}: ${path}"
				fi
			else
				errors="${errors}\nMigration files are append-only: ${status} ${path}"
			fi
			;;
		db/bootstrap/*.sql|db/seed/*.sql)
			baseline_change=1
			;;
	esac
done <<EOF
${changes}
EOF

if [ -n "${errors}" ]; then
	printf '%b\n' "${errors}" >&2
	exit 1
fi

if [ "${baseline_change}" = "1" ] && [ "${added_migration}" = "0" ]; then
	echo "Bootstrap or seed SQL changed without a new db/migrations/*.sql file." >&2
	exit 1
fi

python3 deploy/ec2/migrate-rds.py --validate-only
echo "Database migration check passed."
