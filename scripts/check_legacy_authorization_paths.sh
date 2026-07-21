#!/usr/bin/env bash
set -euo pipefail

mode="${1:---report}"
if [[ "$mode" != "--report" && "$mode" != "--enforce-zero" ]]; then
	printf 'usage: %s [--report|--enforce-zero]\n' "$0" >&2
	exit 2
fi

pattern='has_permission|require_permission|ctx\.is_admin|RequireAdmin|require_admin|can_access_user_admin|permission_contract|set_org_context|can_modify'
matches="$(rg -n "$pattern" crates/libs crates/services/web-server/src \
	-g '!**/tests/**' \
	-g '!**/examples/**' \
	-g '!**/target/**' || true)"

if [[ -n "$matches" ]]; then
	printf '%s\n' "$matches"
	if [[ "$mode" == "--enforce-zero" ]]; then
		exit 1
	fi
fi
