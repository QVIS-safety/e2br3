# Client Requirements Run Plan

Source tracker: [client_requirements_todo.md](client_requirements_todo.md)

This file groups the remaining open `[ ]` and partial `[-]` tracker entries into coherent implementation runs. Use it as execution planning, not as replacement ground truth. Before starting any run, verify the linked tracker requirements against backend, frontend, tests, DB schema, API docs, and generated OpenAPI.

## Run Status Rules

- `[ ]` not started
- `[-]` in progress or partially completed
- `[x]` verified complete and reflected back into `client_requirements_todo.md`

## Progress Snapshot

- Baseline tracker count at creation: 99 done, 28 partial, 111 open, 238 total checklist entries.
- Baseline remaining tracker entries: 139 open-or-partial checklist entries.
- Baseline run plan: 45 remaining implementation runs.

## P0 Runs

- [x] Run 01 - Product naming, final app title, and configurable display strings. Completed with backend `/api/app/branding`, backend/static console branding, Next.js public-env branding helper, visible title/sidebar/login/dashboard copy updates, OpenAPI title cleanup, env defaults, and targeted backend/frontend tests.
- [x] Run 02 - QVIS, Client, Organization, and Sender source-of-truth decision and backend/admin cleanup. Completed with routing sender options sourced from INFO sender masters plus case headers, admin sender scope assignment using backend sender identifiers instead of template UUIDs, sender presave identifier typing/schema support, and backend/frontend targeted tests.
- [x] Run 03 - Sender defaults, organization/client linkage, and sender ownership authorization policy. Completed with persisted org-level sender default normalization, INFO UI default filtering based on master data, company sponsor-admin sender-scope assignment blocking per `roles.csv`, and sender/case ownership regression tests.
- [x] Run 04 - Idle session system configuration, warning lead time behavior, and admin controls. Completed with backend validation for system-level idle/warning settings, authenticated user readback, frontend timer normalization, and background refresh that no longer resets idle activity.
- [x] Run 05 - Global `QC` / `QCed` terminology cleanup across backend responses, filters, frontend copy, and tests. Completed dashboard, submission, admin-role copy, lifecycle filter labels, and default backend workflow description cleanup while preserving internal compatibility status codes.
- [x] Run 06 - Global menu placement for `DATA`, user info, and logout. Completed with sidebar footer user identity fallback/role display, consistent `Log out` wording, and a regression test that keeps `DATA` top-level and system-admin-only.
- [x] Run 07 - Permission model finalization for QC/lock wording, admin bypass policy, and workflow role edge cases. Verified and closed with audited sponsor-admin workflow override policy, locked/QCed edit blocking, workflow role/user/read-only edge-case tests, and frontend QC/lock wording checks.
- [ ] Run 08 - Role/routing/scope frontend UAT cleanup for the static Role and Routing Console.
- [ ] Run 09 - INFO, IMPORT, EXPORT, and SUBMISSION product/sender/study visibility UAT and remaining scope gates.
- [ ] Run 10 - Manual/imported case save parity page-level UAT and removal of normal-save import/batch/header noise.
- [ ] Run 11 - Save reason/comment policy for compliance-sensitive case updates beyond delete/status transitions.
- [ ] Run 12 - Delete confirmation copy and final no-password-delete UAT.
- [ ] Run 13 - Case list export history from the case area with file, status, error, time, and user.
- [ ] Run 14 - Follow-up draft creation from an existing case, including `C.1.1`, `C.1.2`, empty `C.1.5`, and source prefill.
- [ ] Run 15 - Case-to-export/submission deep link with source case preselected and prefilled.
- [ ] Run 16 - Appendix selector top-level placement, duplicate selector removal, and duplication-check profile cleanup.
- [ ] Run 17 - MFDS/FDA/ICH regional rendering verification and removal of invalid regional fields.
- [ ] Run 18 - Required date/null-flavor re-audit for `C.1.2`, `C.1.4`, `C.1.5`, `E.i.4`, AE start dates, and sampled full-form gaps.
- [ ] Run 19 - E2B business-rule re-audit across all CASE tabs, including duplication-check report-type matrix.
- [ ] Run 20 - Validation markers and user-facing warning terminology across section/subsection red dots and blocking/non-blocking copy.
- [ ] Run 21 - Date-picker frontend behavior: English locale, future-date blocking, partial/UK-style requirements where applicable.
- [ ] Run 22 - Repeatable CASE line-list UI gaps for remaining `r`, `i`, and `k` structures.
- [ ] Run 23 - CASE field-level `...` actions: audit trail, notation, and clear/delete value.
- [ ] Run 24 - CASE audit trail payload/UI: timestamp, user, field, value, null flavor, notation, reason, newest first.
- [ ] Run 25 - Unsaved-change prompts before navigation away from case pages.
- [ ] Run 26 - Country input standardization to ISO 3166-1 alpha-2 list behavior.
- [ ] Run 27 - MedDRA UX: release/version selection, LLT search, code+term display, and cross-section consistency.
- [ ] Run 28 - WHO-Drug and UCUM data coverage, including LB `F.r.3.3` and MFDS DG WHO-DD behavior.
- [ ] Run 29 - Duplication-check partial-signature behavior and Product ID loaded-value correctness.
- [ ] Run 30 - Export/submission receiver model separation from INFO receiver master data.
- [ ] Run 31 - Remove or relocate Message Header / Receiver Information from SD if still present.
- [ ] Run 32 - Submission history details: batch result, message result, acknowledged date, ICSR count, and data file link.
- [ ] Run 33 - Submission/export queue and history columns, including event column and requested queue metadata.
- [ ] Run 34 - Export/submission advanced search-based case selection by page/section, field, operator, and value.
- [ ] Run 35 - Export/submission simple filters: sender, case number, and ack accept status if required.
- [ ] Run 36 - Excel line-list export.
- [ ] Run 37 - Imported-case export behavior for incomplete or mismatched sender/receiver/header values.
- [ ] Run 38 - Receiver routing enforcement for authority/report type, `N.1.4`, and `N.2.r.3`.
- [ ] Run 39 - Review/RE timeline receiver data loading.
- [ ] Run 40 - INFO list and record UX: labels, required markers, notation placement, header filters, deleted visibility, row-click edit.
- [ ] Run 41 - INFO field-level audit trail access aligned with CASE behavior.
- [ ] Run 42 - Study/Product master data: product semantics, multi-select, MFDS/FDA study regional fields, and CASE mapping decision.
- [ ] Run 43 - Narrative element-ID composition scope decision and implementation or explicit deferral.
- [ ] Run 44 - Admin role/user UX: `Role Setting` vs `Role & Privilege`, workflow UX, user deletion, one-admin rule, large-list scope picker, comments-only decision.
- [ ] Run 45 - Admin/user and P1 UAT closeout: organization screen removal, user edit/delete flow, dashboard appendix behavior, To Do workflow relation, case-list header filters, section-specific rechecks, import timestamp/error-detail duplicate tracker cleanup.
