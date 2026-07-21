#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <frontend-repository>" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
frontend_root="$(cd "$1" && pwd -P)"

if [[ ! -e "$frontend_root/.git" || ! -f "$frontend_root/package.json" ]]; then
  echo "not a frontend repository: $frontend_root" >&2
  exit 2
fi
if [[ -L "$frontend_root/lib" || -L "$frontend_root/lib/auth" ]]; then
  echo "refusing symlinked frontend authorization directory" >&2
  exit 2
fi

output_parent="$(cd "$frontend_root/lib/auth" && pwd -P)"
case "$output_parent/" in
  "$frontend_root/"*) ;;
  *)
    echo "authorization output escapes frontend repository" >&2
    exit 2
    ;;
esac

output="$output_parent/generated-authorization.ts"
if [[ -L "$output" ]]; then
  echo "refusing symlinked authorization output: $output" >&2
  exit 2
fi

temporary_output="$(mktemp "$output_parent/.generated-authorization.ts.XXXXXX")"
cleanup() {
  if [[ -e "$temporary_output" ]]; then
    rm -f -- "$temporary_output"
  fi
}
trap cleanup EXIT

cargo run --quiet --manifest-path "$repo_root/Cargo.toml" \
  -p lib-core --example export_authorization_contract -- "$temporary_output"
chmod 0644 "$temporary_output"
mv -f -- "$temporary_output" "$output"
trap - EXIT
