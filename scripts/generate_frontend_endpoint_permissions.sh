#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <frontend-generated-endpoint-permissions.ts>" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_TARGET_DIR="$repo_root/target-permission-generator" \
  cargo run --quiet --manifest-path "$repo_root/Cargo.toml" \
  -p web-server --example export_permission_contract -- "$1"
