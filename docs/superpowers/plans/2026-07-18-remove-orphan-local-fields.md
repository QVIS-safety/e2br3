# Remove Orphan Local Fields Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Blocking gates:** Three decision points below (D1–D3) require explicit product sign-off BEFORE Phase 1 starts. Do not begin implementation with any gate unresolved.

**Goal:** Delete the 13 `local_only` registry fields whose app wiring is broken or absent — 8 half-wired (save-only or load-only, no form) and 5 fully unwired — including their backend columns, REST surface, frontend vestiges, and registry rows, while keeping all four registry validation modes green.

**Decision basis (2026-07-18 audit):** Fields were judged by app integration only (presave / case-edit load / save / form binding), not by XML participation. Scans ran against backend `origin/dev` and frontend `origin/main` via git refs. All 13 backend columns ARE written by the XML import path, so deletion destroys imported values for these fields — that is accepted, subject to gates D1–D3.

**Key simplification:** `DrugRecurrenceInformation`'s entire payload is exactly 4 of the 13 columns (`rechallenge_action`, `reaction_meddra_version`, `reaction_meddra_code`, `reaction_recurred`); the rest is id/audit plumbing. Drop the whole table, its REST routes, and the frontend subresource instead of deleting columns. Live recurrence data continues to flow through `DrugReactionAssessment.recurrence_action` (a `complete` registry row with a SectionG input).

**Tech Stack:** Rust/Axum/SQLx/PostgreSQL (backend repo, branch off `dev`), Next.js/TypeScript (frontend repo `../frontend/E2BR3-frontend`, branch off `main`), registry validator (`registry/tools/validate.py`, 4 modes).

## The 13 Fields

| # | Registry row | Backend column | Wiring state |
|---|---|---|---|
| 1 | `D.local.patientGivenName` | `PatientInformation.patient_given_name` | save-only; no load, no form (always sends undefined) |
| 2 | `D.local.patientFamilyName` | `PatientInformation.patient_family_name` | save-only; no load, no form |
| 3 | `G.k.local.supplemental.genericName` | `DrugInformation.drug_generic_name` | load-only; save path never sends it |
| 4 | `G.k.local.parentDosageText` | `DrugInformation.parent_dosage_text` | save-only (`drug.ts:197`); no load, no form |
| 5 | `G.k.local.dosage.firstAdministrationTime` | `DosageInformation.first_administration_time` | save-only; **load missing = latent clobber bug** (gate D1) |
| 6 | `G.k.local.dosage.lastAdministrationTime` | `DosageInformation.last_administration_time` | save-only; load missing (gate D1) |
| 7 | `G.k.local.recurrence.reactionRecurred` | `DrugRecurrenceInformation.reaction_recurred` | save-only via `drug-relatedness.ts`; no load, no form (gate D2) |
| 8 | `G.k.local.recurrence.rechallengeAction` | `DrugRecurrenceInformation.rechallenge_action` | save-only; import-side duplicate of `recurrence_action` |
| 9 | `G.k.local.rechallenge` | `DrugInformation.rechallenge` | REST-exposed only; frontend has zero references outside `app/dg-preview` |
| 10 | `G.k.local.recurrence.meddraVersion` | `DrugRecurrenceInformation.reaction_meddra_version` | no frontend code at all |
| 11 | `G.k.local.recurrence.meddraCode` | `DrugRecurrenceInformation.reaction_meddra_code` | no frontend code at all |
| 12 | `G.k.local.assessmentRecurrence.meddraVersion` | `DrugReactionAssessment.recurrence_meddra_version` | no frontend code at all |
| 13 | `G.k.local.assessmentRecurrence.meddraCode` | `DrugReactionAssessment.recurrence_meddra_code` | no frontend code at all |

## Decision Gates (resolve before Phase 1)

- [ ] **D1 — first/lastAdministrationTime (rows 5–6):** these are one missing-load fix away from being healthy fields (G.k.4.r.4/5 time precision). Confirm time-of-day precision is NOT needed; otherwise pull them from this plan and fix the load path instead.
- [ ] **D2 — reactionRecurred (row 7) / table drop:** dropping `DrugRecurrenceInformation` consolidates all recurrence data onto `DrugReactionAssessment.recurrence_action`. Confirm drug-level (non-assessment) recurrence records are not needed.
- [ ] **D3 — patient names (rows 1–2):** `patient_given_name`/`patient_family_name` are emitted into D.1 XML (`export/sections/d.rs:177-178`) and populated by import. Deletion destroys stored names in existing cases and changes export output. Confirm the privacy stance (delete) and whether a pre-migration snapshot/archive is required.

## Global Constraints

- Backend column removal and registry row removal MUST land in the same commit — `--strict-backend-inventory` fails in both directions (column-without-row and row-without-column).
- Frontend PR deploys before the backend PR: the backend removes the `/recurrences` REST routes, so the frontend must stop calling them first.
- Backend branches from `dev` (NOT `main` — main is a stale ancestor ~156 commits behind). Frontend branches from `main` in `../frontend/E2BR3-frontend`; note the local frontend checkout may sit on a WIP branch with uncommitted work — do not discard it.
- The dev database is externally initialized (`SKIP_DEV_INIT=1`); bootstrap SQL edits do not reach it. Apply the drop statements manually or refresh the DB, and note deploy environments need the same migration.
- Every production change follows red-green TDD and is committed at its task boundary.
- Registry validation must stay green in all four modes after Phase 2 (`validate.py`, `--strict-backend-inventory`, `--strict-dictionary`, `--strict-frontend-inventory` — the last one against frontend `main`).

---

## File Map

### Frontend repository: `/Users/hyundonghoon/projects/rust/e2br3/frontend/E2BR3-frontend`

- `lib/api/endpoints/cases/subresources/drug-relatedness.ts`: `upsertDrugRecurrences` + `/recurrences` CRUD calls (delete).
- `lib/api/endpoints/cases/subresources/patient.ts:137-138`: `patient_given_name` / `patient_family_name` payload keys.
- `lib/api/endpoints/cases/subresources/drug.ts:197`: `parent_dosage_text` payload key.
- `lib/api/endpoints/cases/subresources/drug-dosage.ts:64,134`: `firstAdministrationTime` / `lastAdministrationTime` mapping.
- `lib/api/endpoints/cases/core/detail.drugs.ts:198,453`: `drugGenericName` load mapping; recurrences merge noted at :367.
- `lib/case-save/pages/direct-page-patch.ts`: `patientGivenName` / `patientFamilyName` patch entries (~:139-144).
- `lib/case-save/pages/DG/save.ts`: caller of `upsertDrugRecurrences`.
- `lib/schemas/e2br3.ts` / `lib/types/e2br3.ts`: schema/type entries for all removed field names (incl. :365-366 patient names, :584 firstAdministrationTime).
- `components/case-form/sections/SectionG.tsx:47`: `firstAdministrationTime` default value.
- `app/dg-preview/page.tsx`: example-page references (`drugRechallengeAction`, `drugRecurrence`, etc.).

### Backend repository: `/Users/hyundonghoon/projects/rust/e2br3/e2br3` (branch off `dev`)

- `crates/libs/lib-core/src/model/drug_recurrence.rs`: delete file (whole model).
- `crates/libs/lib-core/src/model/patient.rs`: drop `patient_given_name`, `patient_family_name` from all structs.
- `crates/libs/lib-core/src/model/drug.rs`: drop `drug_generic_name`, `parent_dosage_text`, `rechallenge` (DrugInformation) and `first_administration_time`, `last_administration_time` (DosageInformation); drop `recurrence_meddra_version`, `recurrence_meddra_code` from `drug_reaction_assessment.rs`.
- `db/bootstrap/04-patient-information.sql`, `db/bootstrap/07-drug-information.sql`: remove columns and the `drug_recurrence_information` table.
- `crates/services/web-server/src/web/rest/routes/cases.rs:453-458`: `/recurrences` routes; `drug_recurrence_rest` handler module; `case_editor_rest/dg.rs:155` `drugRecurrences` aggregation; `openapi.rs` DTOs (incl. :1574,1626 `parent_dosage_text`).
- `crates/libs/lib-core/src/xml/import_runtime/g.rs`, `import_runtime/helpers/g.rs`, `import_sections/g_drug.rs`: recurrence block, `rechallenge`, dosage times, `parent_dosage_text` intake.
- `crates/libs/lib-core/src/xml/import_runtime/d.rs`, `import_sections/d_patient.rs`: patient name intake.
- `crates/libs/lib-core/src/xml/export/sections/d.rs:177-178`: patient-name emission (D.1 output changes).
- `crates/libs/lib-core/src/xml/export/sections/g.rs`: `fmt_ts(start, dose.first_administration_time)` (~:507), recurrence test fixtures (~:970-990).
- `crates/libs/validator/src/`: references to removed columns (per-column grep; e.g. sections/d.rs, sections/g.rs, rule tables).
- `registry/sections/d-patient.json`, `registry/sections/g-drug.json`: delete the 13 rows.
- `registry/tools/validate.py`: remove `DrugRecurrenceInformation` from `BACKEND_MODELS`.
- Also update: `G.k.local.reactionRecurredLegacy` row (`not_applicable`, references the dropped block) — delete or re-note.

---

## Phase 1 — Frontend: stop sending/loading (separate repo, PR to `main`)

- [ ] 1.1 Delete `upsertDrugRecurrences` from `drug-relatedness.ts` and its call in `lib/case-save/pages/DG/save.ts`; remove recurrences merge in `detail.drugs.ts`.
- [ ] 1.2 Remove `patient_given_name`/`patient_family_name` from `patient.ts` payload, the two `direct-page-patch.ts` entries, and schema/type entries.
- [ ] 1.3 Remove `drugGenericName` load mapping (`detail.drugs.ts:198,453`) and type entries.
- [ ] 1.4 Remove `parent_dosage_text` from `drug.ts:197` and types.
- [ ] 1.5 Remove `firstAdministrationTime`/`lastAdministrationTime` from `drug-dosage.ts`, SectionG defaults, schema/types. *(skip if D1 resolves to "fix instead")*
- [ ] 1.6 Clean `app/dg-preview` references to removed names.
- [ ] 1.7 `tsc` clean + jest suites green; update any tests referencing removed names (`__tests__/case-save/*`, `field-matrix`/`mutation-matrix` DTO wiring tests likely).

## Phase 2 — Backend + registry (branch off `dev`, single PR, atomic)

- [ ] 2.1 Drop `DrugRecurrenceInformation`: delete `drug_recurrence.rs`, REST routes/handler, `dg.rs` aggregation, openapi DTOs, import-runtime writes, export fixtures. *(gate D2)*
- [ ] 2.2 Drop `DrugReactionAssessment.recurrence_meddra_version/_code` (model, import writes, fixtures).
- [ ] 2.3 Drop `DrugInformation.rechallenge`, `.drug_generic_name`, `.parent_dosage_text` (model, REST/openapi, import/export touchpoints, validator refs).
- [ ] 2.4 Drop `DosageInformation.first/last_administration_time`; export `fmt_ts` falls back to date-only. *(gate D1)*
- [ ] 2.5 Drop `PatientInformation.patient_given_name/_family_name`; remove D.1 name emission in `export/sections/d.rs` and import intake. *(gate D3; snapshot first if required)*
- [ ] 2.6 Update `db/bootstrap/*.sql`; write `DROP TABLE drug_recurrence_information` + `ALTER TABLE ... DROP COLUMN` migration notes for externally-managed DBs.
- [ ] 2.7 Same commit: delete the 13 registry rows; remove `DrugRecurrenceInformation` from `BACKEND_MODELS`; handle `G.k.local.reactionRecurredLegacy`.
- [ ] 2.8 Update export roundtrip/golden fixtures and validator rule tables that referenced removed fields.

## Phase 3 — Verification

- [ ] 3.1 Backend: `cargo check --all-targets`; workspace tests (note: dev CI `Test` job was already red before this work — compare failures against that baseline, do not absorb them).
- [ ] 3.2 Registry: all four validate.py modes green + `python3 -m unittest discover -s registry/tools -p "test_*.py"`.
- [ ] 3.3 Frontend: `tsc` + test suites green.
- [ ] 3.4 Export diff review: D.1 no longer emits given/family names; G.k.4.r.4/5 emit date-only; no recurrence fragment emitted from the dropped table. Confirm against XSD (`E2BR3_XSD_PATH` gate in CI job).
- [ ] 3.5 Round-trip: import an MFDS/FDA sample containing the removed elements → confirm clean ingest (values discarded, no crash) → export → validate.

## Risks

- **Imported-data destruction:** all 13 columns are written by XML import today; migration deletes those stored values (patient names most sensitive — gate D3 snapshot).
- **Export output changes** (D.1 names, G.k.4.r date precision) — downstream consumers/golden tests must be updated together.
- **API compatibility:** `/recurrences` endpoints disappear; Phase 1 must be deployed first.
- **Registry conflict rows unaffected:** the `FDA.G.k.12.r.*` conflict cluster and `deviceCharacteristic.*` local rows are explicitly OUT of scope here (kept; see keep-analysis below).

## Appendix — Keep analysis for the remaining 20 local rows (2026-07-18)

App-wiring evidence per field (scanned against backend `origin/dev` and frontend `origin/main`; "form" counts rendered inputs OR file-driven `setValue` population).

### A. Structurally local — justified, keep as-is (7)

| Row | App wiring | Why it must be local |
|---|---|---|
| `C.1.6.1.r.2.local.mediaType` | load 2 / save 3 / form setValue | HL7 ED XML **attribute** of official C.1.6.1.r.2; the dictionary has no codes for attributes |
| `C.1.6.1.r.2.local.representation` | load 3 / save 3 / form setValue | same (ED attribute) |
| `C.1.6.1.r.2.local.compression` | load 3 / save 3 / form setValue | same (ED attribute) |
| `C.4.r.2.local.mediaType` | load 2 / save 3 / form setValue (SectionLiterature) | ED attribute of official C.4.r.2 |
| `C.4.r.2.local.representation` | load 3 / save 3 / form setValue | same |
| `C.4.r.2.local.compression` | load 3 / save 3 / form setValue | same |
| `G.k.local.dosage.frequencyValue` | load 1 / save 4 (`drug-dosage.ts`) | numeric period component (HL7 periodValue) of G.k.4.r.3; official G.k.4.r.2 maps separately to `DosageInformation.number_of_units` |

### B. Duplicates official functionality — keep now, revisit (12)

| Row | App wiring | Verdict |
|---|---|---|
| `FDA.G.k.local.deviceCharacteristic.*` ×8 (code, codeSystem, codeDisplayName, valueType, valueValue, valueCode, valueCodeSystem, valueDisplayName) | `deviceCharacteristics` wired end-to-end: `CaseEditor.tsx`, `SectionG.tsx`, `detail.drugs.ts` (load), `lib/case-save/pages/DG/save.ts` (save), `pathOwnership.ts` | **Inverted mapping.** XML export emits FDA device data from `DrugDeviceCharacteristic` (`export/sections/g.rs`, `export/roundtrip/g_drug.rs`), yet the official `FDA.G.k.12.r.*` rows (15, status `conflict`) point at the `DrugInformation.fda_device_info_json` carrier. Resolving those conflicts should re-point the official rows at `DrugDeviceCharacteristic` columns/codes and absorb these 8 local rows |
| `C.local.sourceDocumentBase64` / `MediaType` ×2 | load 2 / save 3 / form setValue (SectionC1 upload) | App-local upload path (`SourceDocument` table) running parallel to official C.1.6.1.r (`documentsHeldBySender`). Feature duplication — consolidation candidate; merging the two upload paths would retire these rows (plus the `complete` `C.local.sourceDocumentName`) |
| `G.k.local.supplemental.brandName` | **backend presave.rs 5 hits + frontend presave 7 hits**, `detail.drugs.ts` load, shown in duplication-check page and admin | Duplicates official product naming (G.k.2.2 / G.k.2.3.r) but is the key of the presave product-lookup feature — keep as an app feature |
| `G.k.local.supplemental.dosageText` | REST 11 / load 2 / save 5 | Drug-level single text vs official G.k.4.r.8 which is per-dosage-row; different shape, keep |

### C. Pure app feature — justified (1)

| Row | App wiring | Why local |
|---|---|---|
| `E.local.includedInEmaImeList` | load (`detail.reactions.ts:236`) / save (`reactions.ts:161`) — round-trips through the editor, no input | EMA IME (important medical events) list flag; not an element in the ICH, FDA, or MFDS dictionary |
