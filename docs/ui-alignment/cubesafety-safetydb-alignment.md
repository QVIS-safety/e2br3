# CubeSafety / SafetyDB UI Alignment Log

This log records SafetyDB frontend UI changes made after comparing against the local manually captured CubeSafety workflow reference.

Local reference pack:

- `docs/workflows/cubesafety-admin/admin-panel-workflow.md`
- `docs/workflows/cubesafety-admin/screenshots/`

The local reference pack is intentionally ignored by git because it contains manually captured screenshots.

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
