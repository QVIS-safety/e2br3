# UI to XML Roundtrip Issues (2026-02-24)

## Progress Update (2026-02-25)

- `DH` one-off rerun: `ok=true`, `missingCount=0`, `exportStatus=200`
- `NR` one-off rerun: `ok=true`, `missingCount=0`, `exportStatus=200`
- `DG` one-off rerun: still blocked by `exportStatus=400` (schema ordering/content issue in `G` section v2 export path), so missing-field assertions cannot proceed until export is valid.

### DG Unblocked (latest)

- `DG` one-off now reaches `exportStatus=200` after schema fixes in G patch/export flow.
- Current DG status: `changed=38`, `missingCount=22` (export-valid, now purely mapping gaps).

Latest DG export-400 details (after fixes applied):

- `Element '{urn:hl7-org:v3}subjectOf': This element is not expected. Expected is one of (consumable, performer, author, location, outboundRelationship1, outboundRelationship2, inboundRelationship).`

## Scope

Exhaustive per-tab UI automation attempted to edit visible fields on imported cases, then:

1. Save
2. Mark validated (internal validator route)
3. Export XML
4. Assert edited sentinel values are present in exported XML

Source artifacts:

- `/tmp/ui_tab_checks_results.ndjson`
- `/tmp/ui_tab_CI.log` ... `/tmp/ui_tab_NR.log`

## Summary

- Latest rerun tabs checked: 11 (`CI RP SD LR SI DM DH AE LB DG NR`)
- Discovered fields: 127
- Attempted edits: 123
- Changed in UI: 116
- Edited values missing in exported XML: 54
- `CI/SI` export status: 200 (no C.1.2 miss in targeted CI/SI rerun)
- Full-run notes: `DG` tab had an import timeout in this pass (rerun needed for stable DG count)

## Per-tab Results (Latest Full Rerun)

| Tab | Discovered | Attempted | Changed | Export | Missing in XML |
| --- | ---: | ---: | ---: | ---: | ---: |
| CI | 14 | 14 | 14 | 200 | 1 |
| RP | 15 | 15 | 15 | 200 | 12 |
| SD | 25 | 25 | 25 | 200 | 9 |
| LR | 0 | 0 | 0 | 200 | 0 |
| SI | 14 | 14 | 14 | 200 | 0 |
| DM | 28 | 25 | 18 | 200 | 14 |
| DH | 8 | 8 | 8 | 200 | 8 |
| AE | 19 | 18 | 18 | 200 | 6 |
| LB | 0 | 0 | 0 | 200 | 0 |
| DG | 0 | 0 | 0 | 0 (timeout) | 0 |
| NR | 4 | 4 | 4 | 200 | 4 |

## Root Cause Found (Export 400)

For both CI and SI failing cases:

- Export error body reports:
  - `Element '{urn:hl7-org:v3}value', attribute 'value': [facet 'pattern'] The value '1' is not accepted by the pattern 'true|false'.`

Likely first fix target:

- `FDA.C.1.12` (`combination_product_report_indicator`) is written into a `BL` node.
- UI/DB can hold coded values (e.g. `1`, `2`) while XML expects `true|false`.

## Missing Field Buckets (latest)

- Reporter (`RP`): C.2.r.*
- Sender (`SD`): C.3/A.1.5.*
- Patient (`DM`): D.* and FDA.D.*
- Drug history (`DH`): D.8.r.*
- Reactions (`AE`): E.i.1.1a / E.i.1.2 / E.i.2.1a / FDA.E.i.3.2h / E.i.4 / E.i.5
- Drugs (`DG`): rerun needed (latest full sweep timed out before DG import)
- Narrative (`NR`): H.1 / H.2 / H.4 / H.5.r.3

## Fix Plan (One by One)

1. Fix export-400 boolean serialization in section C (`CI`/`SI` blocker). ✅
2. Re-run CI and SI; confirm export 200. ✅ (`CI` export 200, `SI` export 200 on fresh case)
3. Fix C.1.1 persistence path (`safety_report_id` must update on case row). ✅
4. Fix C.1.2 datetime precision and persistence path. ✅
4. Remaining triage by highest impact:
   - DG (26)
   - DM (14)
   - RP (12)
   - SD (9)
   - DH (8)
   - NR (4)
   - AE (3)
5. For each bucket:
   - identify DTO -> XML mapping path
   - patch exporter
   - add/adjust regression test
   - rerun tab check

## Completed Fixes

### A) Export-400 boolean serialization (C.1.12 / BL)

- File: `crates/libs/lib-core/src/xml/raw/patch.rs`
- Change: normalize `combination_product_report_indicator` (`1/yes/true`, `2/no/false`) into valid BL lexical values (`true|false`) before writing XML `@value`.
- Outcome:
  - `CI` export no longer fails with schema error `pattern 'true|false'`.

### B) C.1.1 save path (case safety_report_id sync)

- File: `frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`
- Change: when safety report section is dirty, also call `updateCase` with `safety_report_id` so C.1.1 persists to `cases.safety_report_id` (used by exporter).
- Outcome:
  - `CI`/`SI` now persist C.1.1 correctly in stable reruns.

### C) C.1.2 end-to-end datetime fix

- Backend XML patch chain:
  - File: `crates/libs/lib-core/src/xml/raw/patch.rs`
  - Added `transmission_date_value` / `transmission_date_time` support and made C.1.2 prefer exact 14-digit message datetime.
- Backend exporter wiring:
  - Files:
    - `crates/libs/lib-core/src/xml/export_sections/c_safety_report.rs`
    - `crates/libs/lib-core/src/xml/export.rs`
  - Passed message-header context into section-C patch/export paths.
- Backend persistence fix:
  - File: `crates/libs/lib-core/src/model/message_header.rs`
  - Root cause: `MessageHeaderForUpdate` did not update `message_date`.
  - Added `message_date` to update DTO + SQL update binding.
- Frontend sync:
  - File: `frontend/E2BR3-frontend/components/case-form/CaseFormWizardNew.tsx`
  - Sync `messageHeader.messageDate` from `safetyReportIdentification.transmissionDate` during save and force message-header upsert in that save cycle.

- Verification (latest rerun):
  - Script: `/tmp/run_tab_checks_ci_si.sh`
  - CI: `ok=true`, `missingCount=0`
  - SI: `ok=true`, `missingCount=0`
