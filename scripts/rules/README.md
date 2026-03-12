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

## MFDS PDF -> Draft Manifest

- Script: `scripts/rules/extract_mfds_pdf_manifest.py`
- Purpose: extract a reviewable draft manifest from the MFDS guidance PDF.
- Input: MFDS PDF file path
- Output:
  - text dump from `pdftotext`
  - draft manifest JSON with provenance + confidence
  - summary markdown

### Usage

```bash
python3 scripts/rules/extract_mfds_pdf_manifest.py \
  --pdf "/path/to/mfds_guide.pdf" \
  --txt-out /tmp/mfds_guide.txt \
  --out-json docs/generated/mfds_pdf_manifest_draft.json \
  --out-md docs/generated/mfds_pdf_manifest_draft.md
```

### Notes

- This parser is heuristic and intentionally conservative.
- It is suitable for bootstrap + gap triage, not for legal-grade final rule authority.

## Refine Draft -> Implementable Candidates

- Script: `scripts/rules/refine_mfds_pdf_manifest.py`
- Purpose: filter draft PDF manifest to leaf-level implementable candidates.
- Input: JSON output from `extract_mfds_pdf_manifest.py`
- Output:
  - refined JSON candidate list
  - markdown summary

### Usage

```bash
python3 scripts/rules/refine_mfds_pdf_manifest.py \
  --in-json docs/generated/mfds_pdf_manifest_draft.json \
  --out-json docs/generated/mfds_pdf_manifest_implementable_draft.json \
  --out-md docs/generated/mfds_pdf_manifest_implementable_draft.md
```

## Build MFDS-Specific Manifest JSON

- Script: `scripts/rules/build_mfds_manifest.py`
- Purpose: emit MFDS-only manifest (`mfds`) from local catalog + official KR rule workbooks.
- Input:
  - MFDS core workbook
  - MFDS individual validation workbook
  - local `catalog.rs`
- Output:
  - MFDS-only JSON with:
    - full MFDS rule list from catalog
    - official KR element inventory
    - missing official KR coverage count
    - local-only MFDS elements

### Usage

```bash
python3 scripts/rules/build_mfds_manifest.py \
  --core-xlsx /path/to/core_rules.xlsx \
  --individual-xlsx /path/to/individual_rules.xlsx \
  --out-json docs/generated/manifests/mfds.rules.json
```

## Build ICH-Specific Manifest JSON

- Script: `scripts/rules/build_ich_manifest.py`
- Purpose: emit ICH-only manifest (`ich`) from local canonical catalog, including:
  - ICH case rules
  - ICH XML/XSD/structure/business rules (`ICH.XML.*` and XML-structure rules)
  - inferred phases (`import`, `case_validate`, `export`)
  - optional ICH rows from FDA workbook extraction (`--fda-workbook-json`)

### Usage

```bash
python3 scripts/rules/build_ich_manifest.py \
  --out-json docs/generated/manifests/ich.rules.json
```

```bash
python3 scripts/rules/build_ich_manifest.py \
  --fda-workbook-json docs/generated/manifests/fda.core_regional_rules.v1_7.extracted.2026-03-07.json \
  --mfds-pdf-draft-json docs/generated/mfds_pdf_manifest_implementable_draft_2026-03-07.json \
  --out-json docs/generated/manifests/ich.rules.2026-03-07.json
```

## Build Consolidated FDA Manifest JSON

- Script: `scripts/rules/build_fda_manifest.py`
- Purpose: emit one consolidated `fda.rules.json` from:
  - extracted FDA workbook rows (`profile=fda` only)
  - local catalog runtime FDA rules

### Usage

```bash
python3 scripts/rules/build_fda_manifest.py \
  --workbook-json docs/generated/manifests/fda.core_regional_rules.extracted.2026-03-07.json \
  --out-json docs/generated/manifests/fda.rules.json
```

## Build Rule Binding Inventory (Cross-Layer Coverage)

- Script: `scripts/rules/build_rule_binding_inventory.py`
- Purpose: map manifest rule codes to where they are implemented/referenced across:
  - backend case validators
  - backend XML business validator
  - backend export postprocess
  - frontend runtime/create-gate/UI required flags/path mapping
  - backend/frontend tests
- Outputs:
  - `docs/generated/manifests/rule_binding_inventory.json`
  - `docs/generated/manifests/rule_binding_inventory.md`

### Usage

```bash
python3 scripts/rules/build_rule_binding_inventory.py
```

Optional frontend path override:

```bash
python3 scripts/rules/build_rule_binding_inventory.py \
  --frontend-root ../frontend/E2BR3-frontend
```

## Build Rule Binding Index (Enum-Based)

- Script: `scripts/rules/build_rule_binding_index.py`
- Purpose: convert inventory evidence into normalized per-rule bindings with enums:
  - `rule_phase`: `import | case_validate | export`
  - `binding_kind`: `enforcement | mapping | metadata | test`
  - `surface`: `case_validation | xml_validation | xml_export | frontend_* | ...`
- Output:
  - `docs/generated/manifests/rule_binding_index.json`

### Usage

```bash
python3 scripts/rules/build_rule_binding_index.py
```

## Check Rule Binding Coverage (CI Gate)

- Script: `scripts/rules/check_rule_binding_coverage.py`
- Purpose: fail if any rule in `rule_binding_index.json` has no `enforcement` binding.

### Usage

```bash
python3 scripts/rules/check_rule_binding_coverage.py
```

## Triage Extracted ICH Candidate Pools

- Script: `scripts/rules/triage_ich_extracted_candidates.py`
- Purpose: compare extracted pools (`workbook_ich_rules`, `mfds_pdf_ich_guidance_rules`) to canonical `ich_rules` and classify each row:
  - `covered_exact_code`
  - `covered_exact_element`
  - `needs_new_canonical_rule`
  - non-actionable buckets (`ack`, data-element-only, optional/unspecified guidance)
- Outputs:
  - `docs/generated/manifests/ich.extracted_candidates.triage.json`
  - `docs/generated/manifests/ich.extracted_candidates.triage.md`

### Usage

```bash
python3 scripts/rules/triage_ich_extracted_candidates.py
```
