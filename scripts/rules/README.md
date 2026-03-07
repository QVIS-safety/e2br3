# Rule Import (FDA Excel -> Normalized JSON)

This folder contains source import utilities for validation rule parity.

## Importer

- Script: `scripts/rules/import_fda_excel_rules.py`
- Input: FDA E2B(R3) Excel workbook (`.xlsx`)
- Output: normalized JSON snapshot

### Usage

```bash
python3 scripts/rules/import_fda_excel_rules.py \
  --xlsx /path/to/FDA_E2B_R3_Core_and_Regional_Data_Elements_and_Business_Rules.xlsx \
  --out crates/libs/lib-core/rules/source/fda/core_regional_rules.normalized.json
```

### Output shape

Each normalized record includes:

- `profile`: `fda | ich | mfds | unknown`
- `scope`: `core | profile_overlay`
- `sheet`, `sheet_kind`
- `tag_id_raw`: source tag ID text from sheet row
- `tag_key`: canonical key used for parity checks
- `severity`
- `message`

## Profile Separation

The importer classifies row profile from:

1. Region/profile/jurisdiction columns (if present)
2. Tag ID namespace (e.g. `FDA.*`, `ICH.*`)
3. Sheet name fallback

This keeps FDA-specific rows (`profile=fda`) separate from core ICH rows (`profile=ich`) in one normalized format.

## MFDS Coverage Audit

- Script: `scripts/rules/audit_mfds_rule_coverage.py`
- Inputs:
  - MFDS core workbook (`Core Data Elements and Business Rules.xlsx`)
  - MFDS individual validation workbook (`Individual item validation rules.xlsx`)
- Output:
  - JSON coverage report
  - Markdown summary

### Usage

```bash
python3 scripts/rules/audit_mfds_rule_coverage.py \
  --core-xlsx /path/to/core_rules.xlsx \
  --individual-xlsx /path/to/individual_rules.xlsx \
  --out-json docs/generated/mfds_rule_coverage_audit.json \
  --out-md docs/generated/mfds_rule_coverage_audit.md
```

### Notes

- The audit normalizes known MFDS element-id aliases that are semantically the same:
  - `G.k.2.3.r.KR.1a` == `G.k.2.3.r.1.KR.1a`
  - `G.k.2.3.r.KR.1b` == `G.k.2.3.r.1.KR.1b`
