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
