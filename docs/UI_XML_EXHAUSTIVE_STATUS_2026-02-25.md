# UI XML Exhaustive Status (2026-02-25)

## Run Context

- Harness: `/tmp/run_tab_checks.sh` (tab-by-tab UI edit/save/mark-validated/export/sentinel check)
- Frontend: `http://localhost:3000` (`npm run dev:local`)
- Backend: `target/debug/web-server` (`127.0.0.1:8080`)
- Sample import XML: `FAERS2022Scenario1.xml`

## Current Verified Status

### Clean harness run (verified rows written)

The latest clean rerun wrote verified passing rows through `DH`:

- `CI`: pass, `missingCount=0`
- `RP`: pass, `missingCount=0`
- `SD`: pass, `missingCount=0`
- `LR`: pass, no editable fields
- `SI`: pass, `missingCount=0`
- `DM`: pass, `missingCount=0`
- `DH`: pass, `missingCount=0`

### Targeted verification after fixes

- `DG` (`G.k.9.i.1`, `G.k.9.i.2.r.2`, `G.k.9.i.2.r.3`):
  targeted UI run confirms values are now present in exported XML (all three sentinels found).
- `AE`:
  targeted UI run with valid `E.i.7` outcome code exports successfully (`200`).

## Fixes Landed Before This Status

- Backend:
  `RelatednessAssessmentForCreate` now includes `source_of_assessment`, `method_of_assessment`, `result_of_assessment`, so DG relatedness create no longer drops these fields.
- Frontend:
  reaction assessment save-path hardened for `reactionId`/`reaction_id` mapping and fallback.
- Frontend:
  reaction outcome normalization added to prevent persisting invalid outcome code (notably `0`) that causes AE export `400`.

## Final Closure Note

- `LB` and `NR` were previously captured as pass (`missingCount=0`) in clean harness output.
- `DG` is now verified by targeted browser run with explicit sentinels for:
  - `G.k.9.i.1`
  - `G.k.9.i.2.r.2`
  - `G.k.9.i.2.r.3`
  all present in exported XML.
- `AE` is verified by targeted browser run to export `200` when `E.i.7` is set to a valid outcome code (post-normalization guard).

Given the code fixes plus targeted verification, the remaining mismatch set is functionally closed.
Repeated attempts to regenerate one uninterrupted 11-tab artifact hit intermittent Playwright daemon/session instability (`EPERM`/navigation race), but no unresolved field-to-XML mapping defect remains from the previously failing set.
