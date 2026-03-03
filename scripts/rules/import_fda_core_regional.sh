#!/usr/bin/env bash
set -euo pipefail

XLSX_PATH="${1:-}"
OUT_PATH="${2:-crates/libs/lib-core/rules/source/fda/core_regional_rules.normalized.json}"

if [ -z "$XLSX_PATH" ]; then
  echo "usage: $0 /path/to/fda_core_regional.xlsx [out_json_path]" >&2
  exit 2
fi

python3 scripts/rules/import_fda_excel_rules.py \
  --xlsx "$XLSX_PATH" \
  --out "$OUT_PATH"

echo "wrote snapshot: $OUT_PATH"

