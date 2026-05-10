# CubeSafety / SafetyDB UI Alignment Log

This log records SafetyDB frontend UI changes made after comparing against the local manually captured CubeSafety workflow reference.

Local reference pack:

- `docs/workflows/cubesafety-admin/admin-panel-workflow.md`
- `docs/workflows/cubesafety-admin/screenshots/`

The local reference pack is intentionally ignored by git because it contains manually captured screenshots.

## 2026-05-10 - DM Long-Form Orientation

- Reference: Flow 12; screenshots 14 and 15.
- Aligned:
  - Added an explicit DM long-form marker to the Section D direct form wrapper.
  - Tightened compact long-form subsection anchors for D.7, D.9, and D.10 while keeping all DM fields visible in one direct form without tabs or accordions.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionD.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/case-edit-shell-alignment.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - RED: `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand -t "DM"`
  - GREEN: `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand -t "DM"`
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts __tests__/field-error-banners/patient.test.ts __tests__/section-tabs-red-dot/patient.test.ts __tests__/case-save/patientHistory.coordinator.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - DM still uses the existing SafetyDB field-row internals; sticky in-page navigation or further row balancing can be handled in a later DM refinement if needed.

## 2026-05-10 - CI/RP/SD/SI Row-Level Refinement

- Reference: Flows 7, 8, 9, and 11; screenshots 07, 08, 09, 10, 12, and 13.
- Aligned:
  - Tightened CI direct rows and repeatable identifier tables with dense row-level shell styling while preserving validation bands directly under the relevant fields.
  - Refined RP summary-table-first behavior by compacting summary rows and changing selected reporter detail orientation to the reference-style `No.n` row label.
  - Reduced SD section divider spacing and aligned sender, message header, and receiver fields as direct dense rows without card-like padding.
  - Kept SI study registration rows compact and removed remaining registration helper copy.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionC1.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionC2.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionC3.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionStudy.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/case-edit-shell-alignment.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand -t "CI, RP, SD"`
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts __tests__/field-error-banners/case-identification.test.ts __tests__/field-error-banners/reporter.test.ts __tests__/field-error-banners/sender.test.ts __tests__/field-error-banners/study.test.ts __tests__/section-tabs-red-dot/case-identification.test.ts __tests__/section-tabs-red-dot/reporter.test.ts __tests__/section-tabs-red-dot/sender.test.ts __tests__/section-tabs-red-dot/study.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - CI/RP/SD/SI still use shared SafetyDB field controls and country autocomplete internals; deeper per-control rendering parity can be handled in a dedicated field-control pass if required.

## 2026-05-10 - Case List Screen Alignment

- Reference: Flow 5, screenshot 05.
- Aligned:
  - Confirmed the case list remains a dense full-width table with row navigation, selection, status, date, DG_PRD_KEY, submission/review/lock/delete metadata, and pagination.
  - Removed reliance on any duplicate global searchbar by covering the absence of a search placeholder in the case-list regression test.
  - Added a compact `Clear Filters` action and visible filter icon controls in filterable column headers while preserving the existing inline column filter values.
  - Made `Date of Creation` filterable from its column header.
- Files changed:
  - `frontend/E2BR3-frontend/app/dashboard/cases/page.tsx`
  - `frontend/E2BR3-frontend/__tests__/dashboard/case-list-header-filters.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - `npx jest --runTestsByPath __tests__/dashboard/case-list-header-filters.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - No additional case-list gaps identified in this pass.

## 2026-05-06 - Alignment Workflow Setup

- Reference: local CubeSafety workflow reference captured through Flow 17, screenshots 01-25.
- Aligned:
  - No SafetyDB frontend UI has been changed yet.
  - Added the repo-side alignment log path expected by the `safetydb-ui-alignment` skill.
  - Added git ignore coverage for the local workflow screenshot/reference pack.
- Files changed:
  - `.gitignore`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - `quick_validate.py /Users/hyundonghoon/.codex/skills/safetydb-ui-alignment`
- Remaining gaps:
  - Run `safetydb-ui-alignment` against specific SafetyDB Admin and Case Edit screens, implement UI changes, verify, then append completed alignment entries here.

## 2026-05-07 - Case List

- Reference: Flow 5, screenshot 05.
- Aligned:
  - Expanded the SafetyDB case list to a full-bleed dashboard workspace so the dense table uses the available screen ratio instead of sitting inside the dashboard padding.
  - Removed the placeholder Warning and E-mail Log tabs from the case list header until those surfaces are implemented.
  - Kept the compact table-first case browsing model, row selection, filters, refresh, CSV export, and pagination controls.
  - Removed redundant `list_options` query parameters from the case-list load call so the page uses the backend's stable default list projection.
  - Moved fixed case utility routes ahead of `/cases/{id}` so `/cases/list-view` is not parsed as a case UUID.
- Files changed:
  - `frontend/E2BR3-frontend/app/dashboard/cases/page.tsx`
  - `crates/services/web-server/src/web/rest/mod.rs`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - `npx tsc --noEmit`
  - `npm run build`
  - `cargo test -p web-server --test api list_view -- --nocapture`
  - `npm run lint` could not complete because the existing `next lint` script prompts for ESLint setup under Next.js 15.
  - Dev server restarted on `http://localhost:3003`; Playwright visual inspection was blocked by an existing locked Playwright MCP browser profile.
- Remaining gaps:
  - Warning and E-mail Log can be restored when backed by implemented views.

## 2026-05-07 - Admin Four Tabs

- Reference: Flows 1-4, screenshots 01-04.
- Aligned:
  - Split SafetyDB Admin into the four reference tabs: User, Role, Role & Privilege, and Settings.
  - Added dense User tab utilities for full-text search, clear filters, row count selection, and access-window status visibility.
  - Changed Role to an inline role/description grid while preserving real custom-role create, update, activate/deactivate, delete, and privilege selection paths.
  - Added a Role & Privilege matrix with Menu, Type, Privilege rows and role columns; built-in role cells are read-only and now display backend-provided fixed sponsor-admin menu privileges, while custom role cells use persisted privileges and the existing update API.
  - Preserved the existing Settings workflow/configuration form and audit panels.
- Files changed:
  - `frontend/E2BR3-frontend/app/dashboard/admin/page.tsx`
  - `frontend/E2BR3-frontend/__tests__/admin-users.header-filters.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN through `npx jest __tests__/admin-users.header-filters.test.ts --runInBand` in isolated worktree.
  - Spec compliance review subagent approved after review-loop fixes.
  - Code quality review subagent approved after preventing unsupported built-in privilege synthesis and persisted empty-privilege defaulting.
  - `npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - SafetyDB now exposes authoritative per-menu built-in sponsor-admin privileges through `/api/admin/roles`, and the matrix renders those checked/read-only cells from the backend. Sponsor-admin privilege editing remains intentionally unavailable because `docs/requirements/roles.csv` defines those role names and authorities as fixed.
  - Legacy static roles without persisted backend rows remain visible as read-only fallback columns until sponsor administrators create matching persisted roles and assign menu-level privileges.

## 2026-05-07 - Admin User Table

- Reference: Flow 1, screenshot 01.
- Aligned:
  - Reworked the SafetyDB Admin User tab back to a dense table-first account list.
  - Removed the standalone user searchbar and kept compact top utilities for Full Text, Clear Filters, and Rows.
  - Changed the primary user columns to `No.`, `Name`, `ID(E-mail)`, `Role`, `Phone`, `Start Date`, `End Date`, and `Status`, with header filter icons instead of always-visible controls.
  - Implemented actual per-column filter popovers from the header icons: text filters for Name, ID(E-mail), Phone, Start Date, and End Date; checkbox filters for Role and Status; and Cancel/Delete/Done actions matching the reference workflow.
  - Changed access-window status text to Active/Inactive for the primary list.
  - Kept role values as dense table text during normal viewing, with the role select only appearing while a user row is being edited.
- Files changed:
  - `frontend/E2BR3-frontend/app/dashboard/admin/page.tsx`
  - `frontend/E2BR3-frontend/__tests__/admin-users.header-filters.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - `npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
  - Playwright visual check on `http://localhost:3004/dashboard/admin` confirmed the Role checkbox popover and absence of the standalone searchbar.
- Remaining gaps:
  - Name, Phone, Start Date, End Date, and Status filters are applied client-side over the currently loaded user page because the current users API filter contract only supports backend filtering for email, username, role, and organization fields.

## 2026-05-08 - Admin Role Table

- Reference: Flow 2, screenshot 02.
- Aligned:
  - Reworked the SafetyDB Admin Role tab into a dense editable role catalog with `No.`, required `Role`, `Description`, and a compact audit-trail icon column.
  - Removed the extra `Required` and `Active` columns from the primary Role grid.
  - Rendered built-in and custom role names/descriptions as inline table inputs so the page matches the reference table-first editing model.
  - Kept built-in role fields protected and moved persisted custom-role description saves to inline blur behavior.
  - Removed the embedded new-role privilege matrix from the Role tab so privilege editing remains isolated to Role & Privilege.
  - Moved new-role creation out of the table into a top-right Add Role modal so the catalog contains only persisted rows.
  - Replaced the visible Actions column with the reference-style audit-trail icon column.
- Files changed:
  - `frontend/E2BR3-frontend/app/dashboard/admin/page.tsx`
  - `frontend/E2BR3-frontend/__tests__/admin-users.header-filters.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - `npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
  - Playwright visual check on `http://localhost:3004/dashboard/admin` confirmed the Role tab renders the inline role table, no embedded privilege matrix, top-right Add Role modal entry, and audit icon column.
- Remaining gaps:
  - Role names for persisted custom roles remain read-only because the backend update contract uses role name as the stable identifier; descriptions remain inline editable.

## 2026-05-09 - Admin Role & Privilege Matrix

- Reference: Flow 3, screenshot 03.
- Aligned:
  - Reworked Role & Privilege into a wide permission matrix directly under the admin tab bar.
  - Changed the privilege catalog from generated menu/action combinations to explicit CubeSafety-style rows for HOME, CASE, CASE INFO., DATA, MONITORING, SUBMISSION, SYNC, ADMIN, and E-mail permissions.
  - Added role-level header checkboxes for bulk toggling editable role columns.
  - Kept built-in role privilege cells disabled/muted and custom role cells editable through the existing role update API.
  - Added sticky orientation columns for Menu, Type, and Privilege with horizontal and vertical scrolling for wide role sets.
- Files changed:
  - `frontend/E2BR3-frontend/app/dashboard/admin/page.tsx`
  - `frontend/E2BR3-frontend/__tests__/admin-users.header-filters.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN with focused Role & Privilege matrix tests.
  - `npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
  - Frontend dev server started on `http://localhost:3004`; live admin inspection was blocked because `target/debug/web-server` could not connect to Postgres (`password authentication failed for user "postgres"`).
- Remaining gaps:
  - The backend privilege payload is still menu-key based, so the UI maps several display rows onto stable menu keys and fields instead of a first-class backend permission tuple model.

## 2026-05-10 - Admin Settings

- Reference: Flow 4, screenshot 04.
- Aligned:
  - Reworked the Settings tab into a dense operational configuration form with left-aligned labels, compact inputs, toggles, appendix switches, case identification number composition, and workflow status rows.
  - Added backend-backed settings fields for MedDRA version, IDF version, company logo, orientation, data ordering, import/export toggles, import date update toggles, appendix switches, case-number format fields, and workflow due days.
  - Preserved workflow role validation while adding due-day validation for workflow rows.
- Files changed:
  - `frontend/E2BR3-frontend/app/dashboard/admin/page.tsx`
  - `frontend/E2BR3-frontend/lib/types/api.ts`
  - `frontend/E2BR3-frontend/__tests__/admin-users.header-filters.test.ts`
  - `crates/services/web-server/src/web/rest/admin_settings_rest.rs`
  - `crates/services/web-server/tests/api/case_validation_web.rs`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN with focused frontend Settings test and backend settings contract tests.
  - `npx jest --runTestsByPath __tests__/admin-users.header-filters.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
  - `cargo fmt --all --check`
  - `cargo check -p web-server --tests`
  - `cargo test -p web-server --test api admin_settings -- --nocapture --test-threads=1`
  - `cargo test -p web-server --test api test_workflow_settings_reject_negative_due_days -- --nocapture --test-threads=1`
- Remaining gaps:
  - The case-number format composer stores selected field names as configuration data; case-number generation can consume this contract in a later workflow if needed.

## 2026-05-10 - Case Duplication Check

- Reference: Flow 6, screenshot 06.
- Aligned:
  - Reworked the SafetyDB new-case duplication gate into a dense top-loaded check form with `CASE LIST`, `WARNING`, and `E-MAIL LOG` context tabs.
  - Replaced the card-style intake layout with compact left-label rows, E2B field codes, required markers, NF controls, calendar affordances for date fields, and muted unavailable-match styling for optional source fields.
  - Moved the primary duplicate-check command to a compact floating bottom-right `Check` action while keeping the existing duplicate-check and create-anyway behavior.
  - Preserved appendix selection, intake duplicate API calls, product-template DG key loading, date normalization, future-date blocking, and duplicate warning review states.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/CaseDuplicationCheckPage.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/CaseDuplicationCheckPage.appendices.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN with focused duplication-check layout regression.
  - `npx jest --runTestsByPath __tests__/case-form/CaseDuplicationCheckPage.appendices.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
  - Frontend dev server started on `http://localhost:3004`; Playwright screenshot of `/dashboard/cases/new` was blocked at the app loading screen because the local backend/auth path was not available for that route.
- Remaining gaps:
  - The `WARNING` and `E-MAIL LOG` tabs are present as disabled context tabs because SafetyDB does not yet expose separate new-case warning or e-mail log panels before case creation.

## 2026-05-10 - Case Edit Shell, CI, RP, SD

- Reference: Flows 7, 8, and 9; screenshots 07, 08, 09, and 10.
- Aligned:
  - Expanded the shared case-edit content area from a centered narrow column to a full-width operational workspace.
  - Tightened the case section tab bar by removing explanatory helper text and reducing tab height/icon sizing while preserving red-dot error indicators.
  - Marked CI, RP, and SD pages with explicit dense section shells for continued section-by-section alignment.
  - Reduced CI, RP, and SD header vertical spacing and removed descriptive helper copy that made the sections feel card-like.
  - Updated RP summary table headers toward the reference wording with `Reporter's Given Name` and `Reporter's Organisation`.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/CaseFormLayout.tsx`
  - `frontend/E2BR3-frontend/components/case-form/SectionTabs.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/CI/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/RP/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/SD/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionC1.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionC2.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionC3.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/case-edit-shell-alignment.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN with focused case-edit shell and CI/RP/SD alignment test.
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand`
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts __tests__/field-error-banners/case-identification.test.ts __tests__/field-error-banners/reporter.test.ts __tests__/field-error-banners/sender.test.ts __tests__/section-tabs-red-dot/case-identification.test.ts __tests__/section-tabs-red-dot/reporter.test.ts __tests__/section-tabs-red-dot/sender.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - CI, RP, and SD field rows still use the existing `E2BFormField` internals; deeper row-level styling and RP edit-form column ordering remain for the next section-specific pass.

## 2026-05-10 - Case Edit LR and SI

- Reference: Flows 10 and 11; screenshots 11, 12, and 13.
- Aligned:
  - Marked LR and SI pages with explicit dense section shells so they follow the case-edit alignment structure after CI, RP, and SD.
  - Reworked LR from a generic inline editable table into the reference-style pattern: compact literature summary table first, selected `No.n` detail editor below.
  - Removed LR helper copy and changed the section title to `C.4.r - Literature References`.
  - Kept LR attachment upload/base64 metadata behavior while moving the file control into the selected-record detail panel.
  - Tightened SI header spacing and removed the explanatory `Clinical study information (if applicable)` helper text.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/pages/LR/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/SI/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionLiterature.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionStudy.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/case-edit-shell-alignment.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN with focused LR/SI case-edit alignment test.
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand -t "LR and SI"`
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts __tests__/field-error-banners/study.test.ts __tests__/section-tabs-red-dot/literature.test.ts __tests__/section-tabs-red-dot/study.test.ts __tests__/case-save/literature.coordinator.test.ts __tests__/case-save/study.coordinator.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - SI still uses the existing repeatable registration table and field layout; deeper CubeSafety-style field row alignment remains for the next pass if the reference page requires it.

## 2026-05-10 - Case Edit DM, DH, AE

- Reference: Flows 12, 13, and 14; screenshots 14, 15, 16, 17, 18, and 19.
- Aligned:
  - Marked DM, DH, and AE pages with explicit dense section shells for continued case-edit alignment.
  - Tightened the DM header and removed explanatory helper copy so the long patient form starts directly with E2B content.
  - Changed DH top heading to `D.8.r - Relevant Past Drug History` and removed helper copy from the toolbar/detail header.
  - Trimmed the DH summary table to the reference scan columns: select, No., drug name, MPID, start date, end date, and indication.
  - Changed AE top heading to `E.i - Reaction(s)/Event(s)` and removed the helper copy while preserving the summary table and selected-row detail form.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/pages/DM/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/DH/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/AE/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionD.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionDH.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionE.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/case-edit-shell-alignment.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN with focused DM/DH/AE case-edit alignment test.
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand -t "DM, DH, and AE"`
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts __tests__/field-error-banners/patient.test.ts __tests__/field-error-banners/drug-history.test.ts __tests__/field-error-banners/reactions.test.ts __tests__/section-tabs-red-dot/patient.test.ts __tests__/section-tabs-red-dot/drug-history.test.ts __tests__/section-tabs-red-dot/reactions.test.ts __tests__/case-save/patientHistory.coordinator.test.ts __tests__/case-save/reactions.coordinator.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - DM remains a long direct form with existing field-row internals; deeper two-column balancing and sticky long-form orientation can be handled in a dedicated DM refinement pass.
  - AE still includes SafetyDB's existing country summary/detail field even though the reference summary table focuses on outcome as the last visible column.

## 2026-05-10 - Case Edit LB, DG, NR

- Reference: Flows 15, 16, and 17; screenshots 20, 21, 22, 23, 24, and 25.
- Aligned:
  - Marked LB, DG, and NR pages with explicit dense section shells.
  - Tightened the LB header, removed helper copy, trimmed the summary table to the reference scan columns, and added the selected-row detail title beside the row badge.
  - Tightened the DG header, removed helper copy, and changed the drug summary table to the reference columns: select, No., Drug Role, DG_PRD_KEY, and Product Name.
  - Surfaced the case-level `dgPrdKey` in the DG summary row so the product key is visible before entering the long drug edit form.
  - Updated NR wording to `H - Narrative Case Summary and Other Information` and removed helper copy from the narrative header and repeatable subsection headers.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/pages/LB/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/DG/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/pages/NR/Page.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionF.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx`
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionH.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/case-edit-shell-alignment.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - TDD RED/GREEN with focused LB/DG/NR case-edit alignment test.
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand -t "LB, DG, and NR"`
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts __tests__/field-error-banners/tests.test.ts __tests__/field-error-banners/drugs.test.ts __tests__/field-error-banners/narrative.test.ts __tests__/section-tabs-red-dot/tests.test.ts __tests__/section-tabs-red-dot/drugs.test.ts __tests__/section-tabs-red-dot/narrative.test.ts __tests__/case-save/tests.coordinator.test.ts __tests__/case-save/drugs.coordinator.test.ts __tests__/case-save/narrative.coordinator.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - DG remains a very long selected-row editor; sticky subsection anchors and deeper dosage/causality row balancing can be handled in a dedicated DG refinement pass.

## 2026-05-10 - DG Long-Form Orientation

- Reference: Flow 16; screenshots 22, 23, and 24.
- Aligned:
  - Marked the selected DG drug edit panel with `data-case-long-form="dg"`.
  - Added a compact near-top DG orientation link row for dosage, indication, and drug-reaction/event assessment subsections.
  - Preserved the DG summary table scan columns: select, No., Drug Role, DG_PRD_KEY, and Product Name.
- Files changed:
  - `frontend/E2BR3-frontend/components/case-form/sections/SectionG.tsx`
  - `frontend/E2BR3-frontend/__tests__/case-form/case-edit-shell-alignment.test.ts`
  - `docs/ui-alignment/cubesafety-safetydb-alignment.md`
- Verified:
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts --runInBand -t "DG"`
  - `npx jest --runTestsByPath __tests__/case-form/case-edit-shell-alignment.test.ts __tests__/field-error-banners/drugs.test.ts __tests__/section-tabs-red-dot/drugs.test.ts __tests__/case-save/drugs.coordinator.test.ts --runInBand`
  - `npx tsc --noEmit`
  - `npm run build`
- Remaining gaps:
  - Deeper DG field-row balancing and subsection-specific layout refinement remain outside this orientation pass.
