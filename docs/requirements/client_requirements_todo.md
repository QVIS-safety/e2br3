# Client Requirements TODO

Source: [03.csv](/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/03.csv)

This file is a normalized implementation backlog derived from all requirement CSV exports in this folder:
- [03.csv](/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/03.csv)
- [roles.csv](/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/roles.csv)
- [wf.csv](/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/wf.csv)
- [list UI.csv](/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/list%20UI.csv)

The main review CSV does not have `requirements / role / wf / list description` columns. Its actual structure is:
- `상위분류`
- `하위분류`
- `PV Comments`
- `개발자 Comments`
- review/update metadata

Use this file as the working checklist for implementation. It is organized by priority and implementation area so requirements can be delivered one by one without rereading the raw review log.

Execution run plan: [client_requirements_runs.md](/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/client_requirements_runs.md)

## Source Summary

- `03.csv`: main review backlog by menu/feature with PV comments and follow-up notes
- `roles.csv`: role hierarchy, authority model, and UI/data scope visibility rules
- `wf.csv`: workflow status model, editable-state rules, and sample routing flow
- `list UI.csv`: required line-list/table UI treatment for repeatable sections

## Backlog Rules

- `[ ]` not implemented or still failing UAT
- `[-]` partially implemented and needs recheck
- `[x]` implemented and mainly needs verification only

## Recent Implementation Updates

- [x] Re-read `03.csv` and reconciled the remaining open/partial items into this tracker.
- [x] Case delete is now a compliance-preserving soft delete: deleted cases stay visible, retain history, and are returned by case read/list/lifecycle flows.
- [x] Delete no longer requires password re-entry; it requires `reason_for_change` and records that reason in the audit/compliance context.
- [x] Deleted cases are read-only for content changes so users cannot keep editing a soft-deleted case.
- [x] Manual and imported-case-shaped case saves now share the same backend and page-level save contract without import batch/header noise. Backend parity is covered by the manual/imported case contract tests, and Run 10 added frontend page-save coverage proving imported-case batch/header data is not resaved during a normal public-field update.
- [x] Save/delete compliance final policy is implemented: normal public-field saves remain reason-free, while delete, submitted/nullified/deleted transitions, and central case identity/scope updates require `reason_for_change` and write audit context. Covered by Runs 10-12 and backend case contract tests.

## Latest `03.csv` Reconciliation

These are the remaining implementation gaps found by rereading `03.csv`. Items that are already represented below remain in their original sections; this summary is a quick implementation index.

- [x] Configurable final product/app naming. Runtime branding now defaults to `QVIS Safety` and can be overridden with backend `E2BR3_APP_NAME` / `E2BR3_APP_SHORT_NAME` and frontend `NEXT_PUBLIC_APP_NAME` / `NEXT_PUBLIC_APP_SHORT_NAME`; backend `/api/app/branding`, the static role console, Next metadata/login/sidebar/dashboard labels, OpenAPI title, and env defaults were updated and covered by `test_app_branding_uses_default_name_without_e2br3`, `test_app_branding_uses_configured_name`, and `app-branding`.
- [x] Notation auto-translation decision and implementation if in scope. Deferred out of first-release scope pending a client-approved translation service and compliance policy; current CASE/INFO notation is manual, auditable, and covered by Runs 23, 24, 40, and 41.
- [x] QVIS / Client / Organization / Sender source-of-truth cleanup. Sender routing options now merge INFO sender masters from `presave_templates` with existing case message-header senders, and admin user scope assignment stores the same sender identifier used by backend routing/scope checks instead of the template UUID; covered by `test_routing_profile_sender_options_include_info_sender_masters`, `test_routing_profile_sender_options_follow_role_scope`, and `sender-template-access`.
- [x] System-level idle-session settings and confirmation of `Idle Session Limit` / `Warning Lead Time` behavior. Admin settings now validate `idle_session_minutes >= 5`, `session_warning_minutes >= 1`, and warning lead time less than the idle limit; the authenticated frontend loads those system settings for all users, computes warning/expiry timers from last real activity, and keeps background token refresh from resetting idle activity. Covered by `test_idle_session_settings_are_system_level_and_validated`, `session-timing`, and `admin-users.header-filters`.
- [x] Global `QC` / `QCed` terminology cleanup outside already converted workflow areas. Case header, case list, dashboard notices/cards/actions, submission queue eligibility messages, lifecycle filter labels, admin role/privilege copy, and default workflow descriptions now use `QC`/`QCed` for user-facing lifecycle wording while preserving backend legacy status codes and explicit Review/RE timeline terminology; covered by `case-status-labels`, `submission-filters`, `npx tsc --noEmit`, and backend settings checks.
- [x] Menu placement cleanup for `DATA`, user info, and logout. The global sidebar keeps `DATA` as a top-level system-admin-only menu item, and the footer now consistently shows the signed-in user's resolved display name/email/role with a `Log out` action; covered by `sidebar.menu-placement` and `npx tsc --noEmit`.
- [x] Case page appendix selector UI placement, duplicate appendix selector removal, and MFDS/FDA regional render UAT. Run 16 completed the shared top-level `AppendixSelector`; Run 17 added regional rendering coverage across representative CASE sections and fixed Section G so KR MPID fields render only when MFDS is selected.
- [x] Case save/QC/lock parity for manual and imported cases, including save reason/comments policy. Manual/imported public-field saves share backend and frontend save contracts; QC/lock edit blocking is covered; central case identity/scope updates now require `reason_for_change` and write it to audit logs.
- [x] Follow-up draft creation from an existing case. Run 14 completed source-case follow-up draft creation with C.1.1 reuse, C.1.2 reset, C.1.5 clearing, ID stripping, source prefill, and `followupDraft` coverage.
- [x] Case-to-export/submission deep link with the source case preselected. Run 15 completed case-header deep links into Export / Submission with the case preselected and authority prefilled, covered by `submission-deep-link`.
- [x] Full null-flavor, business-rule, date-picker, validation-marker, and field-action re-audit across CASE. Runs 18-23 covered required-date null flavors, duplication-check business rules, date-picker behavior, validation markers, and field actions; Run 47 fixed C.1 null-flavor save-fields audit identifiers and verified backend validation coverage across C/D/E/F/G/H/N plus frontend validation/UI-binding suites.
- [x] MedDRA / WHO-Drug / UCUM UX and dataset completeness recheck. Run 27 completed MedDRA UX, and Run 28 completed WHO-Drug helper coverage plus UCUM lab-unit coverage for LB `F.r.3.3` and MFDS DG WHO-Drug behavior.
- [x] INFO list behavior: header filters, deleted-row visibility, row-click edit, required markers, notation placement, and field audit trail. Run 40 completed INFO list header filters, deleted/active status visibility, row-click edit, and required-marker/notation placement cleanup; Run 41 added CASE-style INFO field action buttons with notation and recent presave-template audit history.
- [x] Sender default semantics and sender-based authorization/source-of-truth finalization. Sender `Default` is now an organization-level INFO sender master-data flag (`senderDefault`) normalized by the backend to one default sender per org and consumed by the INFO UI instead of localStorage; company sponsor admins are blocked from assigning sender scope per `roles.csv`, and case/sender ownership is covered by routing, CASE update, INFO scope, export/import/submission history scope tests.
- [x] Study/Product master-data semantics, multi-select behavior, MFDS/FDA regional fields, and automatic mapping to CASE if required. Run 42 changed INFO Study product selection to multi-select registered Product records, added Study-side MFDS/FDA regional data capture, and maps supported Study master fields into CASE, including `C.5.4.KR.1`; FDA C.5.5/5.6 values are preserved in INFO master data pending an authoritative CASE/export target.
- [x] Export/submission receiver separation, receiver routing enforcement, graceful export error history, submission history details, search/filtering, and Excel line-list export. Runs 30-38 completed routing receiver separation/enforcement, submission history details, queue filters/search, line-list export, imported-case export behavior, receiver validation, and export error-history handling with targeted dashboard/backend coverage.
- [x] Import history timestamp bug where `Import Date/Time` can show `Invalid date`. Backend import history emits readable/RFC3339 `uploadedAt` values, and the frontend import history table now falls back to `—` for malformed timestamps, covered by `test_import_history_uploaded_at_is_rfc3339` and `import-history.date-format`.
- [x] Admin/User UAT: `Role Setting` vs `Role & Privilege`, workflow UX, user deletion, one-admin rule if required, large-list scope picker, user table filters, `Access Window`, and organization screen removal. Runs 44-45 completed `Roles & Privileges` wording, workflow UX coverage, user delete/edit/comments, one-admin guard, searchable scope pickers, user table filters, no `Access Window`, and organization screen removal.

## P0 Platform, Access, and Naming

- [x] Make the product/app name configurable and replace the temporary title with the final approved naming. Runtime branding now defaults to `QVIS Safety` and can be overridden with backend `E2BR3_APP_NAME` / `E2BR3_APP_SHORT_NAME` and frontend `NEXT_PUBLIC_APP_NAME` / `NEXT_PUBLIC_APP_SHORT_NAME`; backend `/api/app/branding`, the static role console, Next metadata/login/sidebar/dashboard labels, OpenAPI title, and env defaults were updated and covered by `test_app_branding_uses_default_name_without_e2br3`, `test_app_branding_uses_configured_name`, and `app-branding`.
- [x] Normalize the permission model using `roles.csv` and align it across routing, admin, sender access, QC, lock, and workflow. Role/routing/scope/admin alignment is implemented, including menu-level custom-role privileges, workflow role/user validation, company-admin sender-scope blocking, QC/lock wording cleanup, and audited sponsor-admin workflow override policy.
- [x] Implement the top-level role hierarchy from `roles.csv`:
- [x] `System Administrator (system-admin)`: can grant/revoke Safety Database access to sponsor administrators but has no in-database working authority
- [x] `Sponsor Administrator (CRO)`: fixed admin role with full read/edit access, role creation, privilege editing, user-role assignment, and sender/product/study/blind scope assignment
- [x] `Sponsor Administrator (Pharmaceutical Company)`: same full admin pattern, but scope excludes sender-level assignment where the client does not expect it
- [x] `User`: permissions derived from assigned role plus assigned work scope
- [x] Preserve sponsor administrator role names/authority as fixed built-in roles if the client expects them to be non-editable defaults.
- [x] Allow sponsor administrators to create additional roles that can match sponsor-admin-level permissions if the client wants equivalent custom roles.
- [x] Make routing page visibility depend on the signed-in user’s role and allowed organization/sender scope.
- [x] Clarify and fix how QVIS, Client, Organization, and Sender relate so the routing page and admin settings use the same source of truth. INFO sender masters are now exposed to routing before cases exist, while the admin client-user sender picker/checkboxes persist sender identifiers (`senderIdentifier`, message/batch fallback, sender organization fallback) that match backend access scope filtering.
- [x] Apply the UI visibility rules from `roles.csv`. Run 09 verified the remaining INFO/import/export/submission visibility gates, including product-scoped import, export, and submission histories; backend coverage now locks these rules in `test_import_export_submission_histories_follow_product_scope`.
- [x] CRO sponsor administrators see all senders on the routing page and all related data after routing
- [x] Company sponsor administrators follow the stricter sender rule: no unrestricted sender visibility unless sender scope is explicitly assigned
- [x] normal users see only assigned senders on routing
- [x] CASE shows only allowed product/study values within the routed sender scope
- [x] INFO shows only assigned sender/product/study data. Shared case-linked read gates are in place, presave INFO templates filter sender/product/study list and read endpoints by assigned scope, and Run 09 found no separate non-presave INFO master-data screen outside these guarded APIs.
- [x] IMPORT and EXPORT/SUBMISSION show history only for the user’s allowed product scope. Import-history case-linked error download is gated; Run 09 added coverage proving import history, export history, and global submission history hide cases outside the user’s sender/product scope.
- [x] Move idle session settings to system-level configuration and confirm whether `Idle Session Limit` and `Warning Lead Time` are the intended admin controls. The controls are persisted in `/api/admin/settings` as system-level settings, readable by authenticated users for session timers, editable only by safety DB admins, and backend-validated so warning lead time cannot equal/exceed the idle limit.
- [x] Standardize system terminology to `QC` / `QCed` instead of mixed review/validated wording. User-facing dashboard/submission/admin lifecycle copy now displays `QC` or `QCed` rather than `Reviewed`/`Validated`; internal enum values such as `reviewed` and `validated` remain as compatibility status codes.
- [x] Revisit global menu placement, including where `DATA`, user info, and logout should appear. `DATA` remains in the global navigation with system-admin-only visibility, while user identity and `Log out` are anchored together in the global sidebar footer for desktop/mobile sidebar use.
- [x] Decide whether notation auto-translation is required for the first release or should stay in a later phase. Decision recorded as later-phase scope; first release keeps manual notation with audit trail and no automated translation.

## P0 Case Workflow and Save Model

- [x] Fix page-level save so it works for both directly entered cases and imported cases. Backend case-level save parity is covered for manual and imported-case-shaped records, and frontend `caseIdentification.coordinator` now covers imported-case-shaped normal saves without dirty batch/header writes.
- [x] Remove irrelevant batch/header error messages shown during normal case save. Backend case save contract guards against import/batch/header noise in normal case update responses, and frontend backend-field-banner coverage maps message-date validation to CI transmission date instead of surfacing header noise.
- [x] Require save/delete reason and comments for compliance-sensitive actions. Delete requires `reason_for_change` and records it in audit logs; submitted/nullified/deleted transitions require reason and e-signature; Run 11 now requires and audits `reason_for_change` for central case identity/scope updates such as safety report ID changes.
- [x] Remove password re-entry from delete if the client still wants delete confirmation without PW input. `DELETE /api/cases/{id}` requires reason only and does not require e-signature password re-entry; the frontend delete confirmation now uses reason-only copy and calls the DELETE contract without password fields.
- [x] Keep deleted cases visible as soft-deleted rows with clear visual marking and history retention.
- [x] Make case list export history visible from the case area with file, status, error, time, and user. The case Review area loads `GET /api/cases/{id}/exports/history` and renders file, status, error, time, user, plus error-text download; Run 13 added frontend regression coverage in `CaseHeader.appendix-selector.test.ts`.
- [x] Ensure QC/lock actions behave consistently for manual cases and imported cases. Backend blocks content edits for QCed and locked cases even when workflow is enabled; frontend allows QCed cases to proceed to Lock while keeping content read-only.
- [x] Replace ad hoc review state wording with explicit workflow-aware status where the client expects workflow status instead of a generic checked state. Backend workflow status remains separate from QC and Lock; case list, dashboard, admin copy, and submission filters no longer collapse QC, lock, and workflow into one visible status or use generic reviewed/validated wording for QCed cases.
- [x] Implement correct follow-up draft creation from an existing case. `Create Follow-up Draft` opens `/dashboard/cases/new?followupOf=...`, loads the source case, and renders a new unsaved draft with source data cloned and persistence IDs stripped; covered by `followupDraft.test.ts`.
- [x] Reuse the original case as the source. The follow-up draft builder clones the loaded source case instead of starting from blank intake data.
- [x] Set `C.1.1` correctly for the follow-up report. Follow-up drafts keep the source `safetyReportIdentification.safetyReportId` while incrementing the safety report version for the draft.
- [x] Set `C.1.2` to creation time for the new follow-up. Follow-up drafts set `safetyReportIdentification.transmissionDate` and message date to the new UTC creation timestamp.
- [x] Leave `C.1.5` empty initially. Follow-up drafts clear `safetyReportIdentification.dateOfMostRecentInformation` so the new most-recent-information date must be entered for the follow-up.
- [x] Prefill other fields from the source report. The follow-up helper preserves source appendices, sender, reporter, patient, reaction, drug, narrative, and other non-persistence fields while removing case/subresource IDs.
- [x] Make `Open Export / Submission` from a case open the target flow with that case already selected and prefilled. The case header now opens `/dashboard/submission?caseId=...`; the submission queue applies that deep link by showing all eligibility states, preselecting the linked export-eligible case, and choosing an authority present on that case, covered by `submission-deep-link.test.ts` and `CaseHeader.appendix-selector.test.ts`.

## P0 Workflow Model From `wf.csv`

- [x] Add a distinct case `Status` field for workflow, separate from QC and Lock.
- [x] Make workflow configurable from Admin and allow workflow to be turned on/off.
- [x] Keep workflow off by default unless explicitly enabled.
- [x] Ensure every case has exactly one workflow status value.
- [x] Keep `Saved` as the default status when workflow is not configured or not used.
- [x] Enforce the rule that case editing is allowed only in statuses that are configured as editable, with `Saved` as the baseline editable state.
- [x] Support workflow status metadata fields:
- [x] `Status`
- [x] `Role`
- [x] `Due date`
- [x] `Description`
- [x] Support status-based role routing so only users in the configured role can act on the case at that workflow step. Implemented for canonical runtime roles and active custom roles, with an intentional audited safety-db-admin bypass; Runs 07, 08, and 44 completed the surrounding role cleanup and UAT coverage.
- [x] Support the example workflow from `wf.csv` as a configurable template:
- [x] `Saved`
- [x] `To be reviewed`
- [x] `Internal review completed`
- [x] `Finalized`
- [x] Support routing comments and target user assignment during workflow handoff.
- [x] Allow return transitions such as moving a case from review back to `Saved`.

Implemented notes:
- Backend stores workflow fields separately on `cases` and records route history in `case_workflow_events`.
- Backend exposes `GET /cases/workflow/config`, `GET /cases/{id}/workflow/events`, `POST /cases/{id}/workflow/transition`, and `POST /cases/{id}/workflow/assign`.
- Case read payloads include `can_act_on_workflow` and `workflow_block_reason` so the frontend does not infer ownership or editability.
- Admin settings now validate workflow roles against built-in roles plus active custom roles from `app_roles`.
- Frontend admin settings include a workflow status editor, and the case workflow panel separates Assign and Transition actions.

## P0 Case Validation, Business Rules, and Form UX

- [x] Re-audit null flavor support across the entire case form, especially required date fields and conditionally mandatory elements. Run 47 fixed the C.1 null-flavor save-fields audit identifiers, verified the supported null-flavor checklist has save-field coverage, and reran backend validation coverage for C/D/E/F/G/H/N plus frontend UI-binding coverage for C.1/E/F/G null-flavor surfaces.
- [x] Re-audit regional element correctness so KR/FDA fields only appear where valid and nonexistent regional fields are removed. Run 17 added `regional-rendering.test.ts` for ICH-only, FDA-only, and MFDS-only CASE rendering and removed the KR MPID/WHO-Drug block from non-MFDS Section G rendering.
- [x] Recheck E2B business rules across all case tabs, not just the fields already called out in the CSV. Run 47 reran the backend validation integration suite covering C/D/E/F/G/H/N rule contracts and frontend validation mapping/intake matrix suites; remaining client-specific changes should be tracked as new requirements rather than this umbrella audit.
- [x] Make validation warnings reliable at both section and subsection level so red dots and required indicators match real errors. Backend validation exposes stable section, subsection, field_path, and section/subsection issue counts; Run 20 wires frontend section markers to backend section/subsection summaries and verifies all section-tab red-dot suites plus validation integration coverage.
- [x] Replace blocking/non-blocking wording with user-facing terminology the client can understand. Run 20 adds header validation labels `Needs attention`, `Ready for QC`, and `Not checked`, with regression coverage proving internal blocking/non-blocking terminology is not rendered in the case header/layout.
- [x] Remove `Validation profile` from duplication check and handle appendix selection at the top-level case/home flow instead. Run 16 replaced duplication-check validation-profile wording with case-appendix selection, reused the shared top-level `AppendixSelector`, and verified no legacy `validation_profile` payload is submitted.
- [x] Ensure date pickers are consistently English, support partial/UK-style requirements where applicable, and block future dates where required. Backend case validation blocks future dates for covered C/D/E/F/G fields; Run 21 verifies the C.1 calendar is English and disables future days, normalizes UK-style intake dates such as `30/04/2026` to E2B `YYYYMMDD`, preserves partial date precision in shared helpers, and blocks future intake C.1.5/E.i.4 dates before submission.
- [x] Recheck date/null-flavor behavior for required dates called out in `03.csv`, including `C.1.2`, `C.1.4`, `C.1.5`, `E.i.4`, and AE start-date null flavors. Run 18 added frontend binding regressions proving C.1.2/C.1.4/C.1.5 toggle to `NI`, hide date pickers while null-flavored, and E.i.4 toggles to/from `UNK`; backend validation for C.1 required dates, E reaction dates, and sampled F/G null-flavor dates was re-run.
- [x] Make repeatable structures (`r`, `i`, `k`) use line-list/table-style editing instead of long stacked forms. Run 22 re-audited the remaining CASE repeatables and confirmed the listed `r`, `i`, and `k` structures use either simple line-list tables or table + active-row detail views.
- [x] Implement the line-list UI requirements from `list UI.csv` for repeatable sections not already converted. Run 22 confirms all sections named in `list UI.csv` are implemented: C.1.6.1.r, C.1.9.1.r, C.1.10.r, C.2.r, C.4.r, C.5.1.r, D.7.1.r, D.9.4.r, D.10.7.1.r, D.10.8.r, G.k, G.k.2.3.r, G.k.7.r, G.k.10.r, H.3.r, and H.5.r.
- [x] Make field-level `...` actions support:
- [x] Audit trail
- [x] notation where applicable
- [x] clear/delete value for that field
- [x] Make audit trail show create/update timestamp, user, field name, value, null flavor, notation, and reason, newest first. Run 24 updates the CASE Audit Trail tab to request newest-first logs and display Date/Time, User, Field Name, Value, Null Flavor, Notation, and Reason for Change with regression coverage.
- [x] Make unsaved-change prompts reliable before navigation away from a case page. Run 25 adds regression coverage for browser unload and same-window link navigation with unsaved CASE changes, and hardens the link guard to prevent default before queueing the Save/Leave dialog.
- [x] Standardize country inputs to ISO 3166-1 alpha-2 list behavior. Run 26 converts the remaining Study Registration Country field to the shared ISO country autocomplete and verifies CASE country list behavior through UI binding/regional/integration tests.
- [x] Finish MedDRA UX: Run 27 verifies the shared MedDRA autocomplete helper used across CASE sections searches LLT terms, displays term plus code, carries term/level/version metadata for version population, and is covered by `meddra`, `field-bindings`, `regional-rendering`, and CASE integration tests.
- [x] version selection from a list
- [x] LLT-based search
- [x] code display with term
- [x] consistent behavior across all sections
- [x] Finish WHO-Drug and UCUM data coverage and verify all expected values are present, including the LB `F.r.3.3` UCUM list and MFDS DG WHO-DD version/search behavior. Run 28 adds seeded `mg/dL`, `U/L`, and `mmol/L` UCUM units, a shared frontend UCUM option helper with lab fallbacks, converts LB `F.r.3.3` to a controlled UCUM autocomplete, and verifies WHO-Drug code/term/version metadata for MFDS DG autocompletes.

## P0 Repeatable Line-List UI From `list UI.csv`

- [x] Convert these repeatable sections to line-list/table entry where not already implemented:
- [x] `C.1.6.1.r` Documents Held by Sender
- [x] `C.1.9.1.r` Other Case Identifiers
- [x] `C.1.10.r` Linked Report Identification Numbers
- [x] `C.2.r` Primary Source(s) of Information
- [x] `C.4.r` Literature Reference(s)
- [x] `C.5.1.r` Study Registration
- [x] `D.7.1.r` Structured Information on Relevant Medical History
- [x] `D.9.4.r` Autopsy-determined Cause(s) of Death
- [x] `D.10.7.1.r` Structured Information of Parent
- [x] `D.10.8.r` Relevant Past Drug History of Parent
- [x] `G.k` Drug(s) Information
- [x] `G.k.2.3.r` Active Substance(s)
- [x] `G.k.7.r` Indication for Use in Case
- [x] `G.k.10.r` Additional Information on Drug
- [x] `H.3.r` Sender's Diagnosis / Syndrome and/or Reclassification
- [x] `H.5.r` Case Summary Information
- [x] Use two UI patterns from `list UI.csv`:
- [x] simple line-by-line tables when the repeated item has only a few fields
- [x] table + detail view when the repeated item has many fields, similar to `G.k.4.r` structured dosage information

## P0 Duplication Check

- [x] Recheck type-of-report-specific required fields so the matrix is fully applied for spontaneous, study, other, and unknown report types. Run 19 applies the C.1.3 matrix in frontend intake gating and backend duplicate-basis assessment for spontaneous, study, other, and unknown report scenarios.
- [x] Apply the `03.csv` duplication-check required-field matrix by `C.1.3` report type, including spontaneous, study, other, and unknown report scenarios. Non-study reports now require D.1, D.2.2a, D.5, Product ID, E.i.2.1a, E.i.2.1b, and E.i.4; study reports require C.2.r.2.1, C.5.3, D.1.1.4, Product ID, E.i.2.1a, E.i.2.1b, and E.i.4, with null-flavor placeholders treated as missing.
- [x] Lock duplicate-create policy: duplicate hits are hard-blocked at create-from-intake, while incomplete basis without a duplicate hit remains explicit-override only.
- [x] Confirm duplicate detection logic when only part of the duplicate signature matches. Run 29 verifies backend duplicate matching uses patient/reaction and product/event bases correctly, including patient-signature matches across product mismatch and mismatch cases that should not warn.
- [x] Ensure `Product ID` is the loaded value, not a different product label. Run 29 makes duplication-check product template selection prefer loaded identifier fields (`dgPrdKey`, Product ID, MPID, PhPID, authorization number) before display labels and verifies the intake field receives the identifier.
- [x] Treat null-flavor placeholders inside duplication check inputs as missing values.

## P0 Appendix and Regional Behavior

- [x] Move appendix selection to a clear top-level location on the main or case screen. Backend treats `appendices_json` as the authoritative top-level case selection and the frontend now uses a shared top-level case `AppendixSelector` in the case header/intake path with regression coverage proving multi-appendix create/save behavior without legacy `validation_profile`.
- [x] Remove duplicated appendix selection controls when both a direct selector and a dropdown are shown. Run 16 completed the shared top-level `AppendixSelector`, removed legacy validation-profile intake/dropdown behavior, and verified duplicate-check intake no longer submits `validation_profile`.
- [x] Ensure MFDS/FDA regional fields render strictly according to the selected appendix combination. Frontend contract is explicit and now covered by UI-binding regression: base ICH sections hide FDA/MFDS/KR-only fields, FDA selection shows FDA-only fields without MFDS fields, and MFDS selection shows MFDS/KR fields without FDA-only device fields.
- [x] Clarify expected XML behavior for multi-appendix cases and document whether output is one XML, multiple XMLs, or authority-specific export paths. Multi-appendix cases export as separate authority-specific XML outputs; bulk ZIP emits one XML per selected appendix.

## P0 Submission and Export

- [x] Separate receiver concepts cleanly: Run 30 keeps INFO receiver master data as organization/contact details and moves active N.1.4/N.2.r.3 receiver routing entry to the Export / Submission queue, with legacy receiver-template routing fields used only as fallback for existing data.
- [x] INFO receiver master data
- [x] export/submission routing receiver configuration
- [x] Remove receiver identifiers from the wrong INFO location if the client expects them to live only in submission routing configuration. Run 30 removes receiver identifier/routing controls from the INFO receiver master form and adds explicit submission routing receiver identifier inputs.
- [x] Remove or relocate Message Header / Receiver Information from the SD page if they are still present there. Run 31 removes the Message Header and Receiver Information controls from the SD/C.3 page; receiver routing identifiers now live on Export / Submission and SD remains focused on sender details.
- [x] Make export fail gracefully and always record errors in export history with downloadable text details. Failed single-case and bulk XML export attempts now write `error` rows in `xml_export_history` before returning the original validation/export error; error text download remains available through `GET /api/exports/history/{id}/error.txt`, covered by `test_failed_single_export_records_error_history` and `test_export_history_error_details_download_as_text`.
- [x] Finish submission history details. Backend ACK text download is implemented at `GET /api/submissions/{id}/acks/{level}/download`, documented in OpenAPI, and covered by `test_submission_ack_can_be_downloaded_as_text`; Run 32 added API/UI history fields for batch result, message result, acknowledged date, ICSR count, and XML data-file download link, covered by `test_submission_history_includes_latest_ack_time_and_event` and `submission-history-details`.
- [x] batch result data
- [x] message result data
- [x] acknowledged date
- [x] ACK download
- [x] ICSR count
- [x] data file link
- [x] Add the event column and other requested queue/history columns where still missing. Run 33 expanded the Export / Submission queue with Most Recent Info, WW Unique No, Type of Report, Sender, Event (MedDRA), Submission, and Deleted columns, and Run 32 completed the requested Submission History batch/message result columns; covered by `submission-history-details`, `submission-deep-link`, `submission-receiver-routing`, and `npx tsc --noEmit`.
- [x] Add search-based case selection for export/submission beyond simple filters. Run 34 added an advanced queue search with page/section, field, operator, and value controls, wired through `caseMatchesSubmissionAdvancedSearch`; covered by `submission-filters`, `submission-history-details`, `submission-deep-link`, and `npx tsc --noEmit`.
- [x] page/section targeting
- [x] field targeting
- [x] condition operators
- [x] value matching
- [x] Finish the simpler filter set the client explicitly asked for. Run 35 verifies case number, sender, ACK accept status, QC status, lock status, and workflow status filters on the Export / Submission queue; covered by `submission-history-details`, `submission-filters`, `submission-deep-link`, and `npx tsc --noEmit`.
- [x] sender
- [x] case no
- [x] QC status
- [x] lock status
- [x] workflow status
- [x] ack accept status if still required
- [x] Implement Excel line listing export. Run 36 added an Excel-readable `.xls` line-list export for the filtered Export / Submission queue using the requested queue metadata columns; covered by `submission-line-list-export`, `submission-history-details`, `submission-filters`, and `npx tsc --noEmit`.
- [x] Recheck imported-case export behavior when sender/receiver/header values are incomplete or mismatched. Run 37 verified the backend raw-XML export fast path and dirty-section patching behavior (`export_case_xml`, `try_fast_path_export`, C/header patching) and added a dashboard regression proving clean imported raw XML remains exportable while dirty imported XML is excluded until QCed; covered by `test_single_export_rejects_unselected_appendix_profile`, `submission-history-details`, and `npx tsc --noEmit`.
- [x] Confirm authority/report-type-based receiver identifier selection is enforced strongly enough before submission. Run 38 added strict pre-export/pre-submission validation that resolves the selected authority/report type routing and blocks when either `N.1.4` or `N.2.r.3` is missing.
- [x] Recheck receiver templates/routing rules for `N.1.4` and `N.2.r.3` by country/authority and report type. Run 38 verifies explicit routing identifiers, receiver-template routing-rule fallback, report-type normalization, and required receiver identifiers; covered by `submission-receiver-routing`, `submission-history-details`, and `npx tsc --noEmit`.

## P0 Workflow and Receiver Timeline

- [x] Make the receiver data load correctly in the Review/RE timeline area. Run 39 verified `review_receivers_json` hydrates into `reviewReceivers` for the Review/RE receiver due/reported-date table and pinned it with `api.endpoints`; `npx tsc --noEmit` also passes.
- [x] Rebuild WF behavior using the client’s separate workflow sheet rather than only the current screen-level status storage.
- [x] Add stronger backend workflow state-transition enforcement if workflow is meant to control edit/QC/lock permissions. Workflow editability, role ownership, assignee ownership, destination validation, assignment-only events, no-op transition rejection, history persistence, locked-case blocking, QCed edit blocking, and audited sponsor-admin override policy are implemented and covered by workflow permission tests.

## P0 INFO Master Data

### General

- [x] Align INFO page wording, labels, and record naming with the client’s terminology. Run 40 keeps INFO record naming in the list/actions and tightens remaining sender/product form labels around required markers and notation wording.
- [x] Make INFO required markers and notation placement fully consistent with the CASE form pattern. Run 40 keeps required indicators as leading red markers and nests notation controls under their parent INFO fields, including the remaining repeated sender headings and holder/applicant notation field.
- [x] Add table-header filtering for INFO list screens. Run 40 added header-cell filters for name, linked/summary, comments, updated date, and status, covered by `info-page`.
- [x] Make deleted INFO records remain visible as deleted rather than disappearing silently. Run 40 preserves records flagged as deleted in INFO data/top-level API payloads and marks them with a visible Deleted status, covered by `info-page`.
- [x] Make INFO rows open on click for view/edit, matching the client’s line-list expectation. Run 40 made INFO rows keyboard/click open the edit dialog while action buttons retain their own behavior, covered by `info-page`.
- [x] Add field-level audit trail access in INFO screens similar to the requested CASE behavior. Run 41 added INFO field action buttons for record fields with local notation and recent presave-template audit history loaded from `/api/presave-templates/{id}/audit`, covered by `presave-info-audit`.

### Sender

- [x] Continue refactoring sender as the operational source of truth for organization/client linkage. Routing now reads sender masters from INFO `presave_templates`, admin client-user organization resolution continues from linked sender organization metadata, and sender access values are normalized to backend-visible sender identifiers rather than template IDs.
- [x] Confirm how `Default` should work for sender records. `Default` is implemented as one persisted org-level INFO sender master flag (`senderDefault`), not a browser-local setting; creating/updating a sender as default clears the flag from other sender masters in the same organization and is covered by `test_presave_sender_default_is_org_level_singleton`.
- [x] Rework sender-based authorization if the client expects backend-enforced sender ownership across case processing and submission. Backend scope enforcement already filters routing, CASE reads/lists, INFO sender/product/study lists, import history, export history, submission history, and case export/download flows through `case_matches_user_scope`; Run 03 added `test_case_update_requires_matching_sender_scope` to lock CASE update ownership as well.
- [x] Verify sender organization/client linkage against `roles.csv`, including which admin role manages client organization data. `roles.csv` assigns Sender/Product/Study/Blind scope management to CRO/Safety Database Admin, while Pharmaceutical Company sponsor admin manages Product/Study/Blind scope only; backend user create/update now rejects company-admin sender scope assignment and the admin UI disables client/sender-scope assignment for that role, covered by `test_company_sponsor_admin_cannot_assign_sender_scope`.

### Study

- [x] Make product selection come from registered master data with the exact product semantics the client expects. Run 42 replaced the free-text INFO Study product entry with registered Product master record selection while preserving a compatibility product-name summary.
- [x] Support the requested MFDS and FDA study regional elements completely, including `C.5.3` study no/protocol no choice, `C.5.4.KR.1`, `FDA.C.5.5a`, `FDA.C.5.5b`, and `FDA.C.5.6.r`. Product master fields already carried the regional values; Run 42 added the Study-side study no/protocol no selector, MFDS other studies type, FDA IND/Pre-ANDA, and repeatable cross-reported IND capture.
- [x] Allow the multi-select behavior the client requested where applicable, especially product selection from registered master data. Run 42 added multi-select Product master selection to INFO Study records, covered by `study-product-master`.
- [x] Add `Study Registration (C.5.1.r)` repeatable support.
- [x] Extend automatic mapping from study/product master data into relevant CASE fields if the client expects this to be automatic. Run 42 maps supported INFO Study fields into CASE, including study name, sponsor study number, study type, registrations, and `C.5.4.KR.1`; FDA C.5.5/5.6 values remain stored in INFO master data until CASE/export schema adds explicit target fields.

### Narrative

- [x] Keep the structured narrative fields already added, but remove unwanted `Additional Narrative Fields` content if still present. Run 43 removed the deprecated INFO Narrative block label while retaining structured H.2/H.3.r/H.4 fields; top-level narrative payloads already reject stale extra fields such as `case_summary`, and frontend payload docs point summaries to the structured repeatable `/narrative/summaries` endpoint.
- [x] Decide whether full element-ID-based narrative composition is in scope now or explicitly deferred. Run 43 explicitly defers cubeSAFETY-style full element-ID NR Auto composition as future scope per the 2026-04-02 PV comment, while preserving the current structured E2B H-field narrative template behavior.

## P0 Admin and User Management

### Roles and Privileges

- [x] Simplify custom role creation so `role_name` and visible name behave the way the client expects. Custom-role create now accepts `role_name` plus `description` without a separate visible-name input, and defaults backend `display_name` to the normalized role ID for compatibility.
- [x] Replace `Display name` with `Description` if that matches the requested admin UX. The admin console no longer shows or requires a `Display name` field, and a static UI contract guards the wording/payload.
- [x] Clarify and possibly merge `Role Setting` vs `Role & Privilege` if the distinction is confusing to the client. Run 44 keeps one explicit `Roles & Privileges` admin tab and verifies the deprecated `Role Setting` wording is absent.
- [x] Support per-menu permissions for read, edit, QC/review, and lock instead of only coarse role creation.
- [x] Add edit capability for custom roles after creation.
- [x] Reconcile admin UX with the separate client workflow sheet where requested. Workflow status editing is available in Admin Settings, Role/Privilege menu-level controls are consolidated under `Roles & Privileges`, and Run 44 verifies the user-facing wording.
- [x] Make Safety Database Administrator a role that can be created/managed by sponsor administrators, as described in `roles.csv`.
- [x] Support delegated role creation such as `PVS`, `PVM`, and other client-defined roles from `roles.csv`.

Implemented notes:
- Role admin APIs now normalize either structured menu privileges or legacy boolean inputs into canonical menu-level privilege records.
- Role list/detail responses expose both canonical privilege maps and compatibility summary booleans (`can_view`, `can_review`, `can_lock`, `can_admin`).
- Role privilege validation rejects unknown menu keys and empty privilege sets.
- Frontend admin role editing now uses a per-menu privilege matrix instead of only coarse `View/Review/Lock/Admin` toggles.

### Create / Edit User

- [x] Username removal and organization scoping are mostly done, but the whole create/edit flow still needs UAT recheck. Run 44 keeps email as the login identifier, preserves sender-derived client organization scoping, and extends user edit to comments while Run 45 retains final full-flow UAT.
- [x] Keep only `Comments` if `Other information` should be removed. Run 44 removed the `Other Information` create/display UI and verifies the Add User form is comments-only.
- [x] Fix start/end date handling if account creation still fails when those values are set.
- [x] Allow sender/product/study scope to be edited after user creation.
- [x] Fix role reassignment and user deletion after save. Role reassignment uses the normalized update payload, and Run 44 adds user-list delete action coverage that calls the delete API after confirmation.
- [x] Add the requested blind-flag behavior if required.
- [x] Enforce the client’s rule that only one admin can hold the relevant admin role, if that is still a hard requirement. Run 44 adds an admin UI guard that prevents assigning a second built-in or admin-capable custom admin role.
- [x] Improve scope selection UX for large sender/product/study lists, likely with dropdown or popup multi-select. Run 44 adds searchable sender/product/study scope pickers to the Add User flow, covered by `admin-users.header-filters`.
- [x] Support scope assignment dimensions from `roles.csv`:
- [x] sender scope
- [x] product scope
- [x] study scope
- [x] blind/non-blind scope where required

### User List and Organization

- [x] Add table-header filtering to the user list. Frontend admin Users table now includes User, Role, and Organization header-cell filters wired to the existing users API filter state, covered by `admin-users.header-filters`.
- [x] Clarify the meaning of `Access Window`. Frontend admin Users table now labels it as active start/end dates, covered by `admin-users.header-filters`.
- [x] Remove any remaining standalone Organization management screen if sender-based organization management is now the intended model. Run 45 removed the standalone Organizations panel from ADMIN; sender-based client organization assignment remains in Add User.
- [x] Recheck user create/edit/delete end-to-end after role reassignment, sender/product/study scope changes, blind flag changes, and start/end date entry. Run 44/45 verified comments-only create/edit, role reassignment guard, searchable sender/product/study scope assignment, access-window handling, and user-list delete action coverage; backend role/scope/date delete contracts were already covered by user API tests.

## P1 Case List and Dashboard Polish

- [x] Reconfirm whether the client still wants the `ICSRs` wording change or wants different copy. Run 45 keeps the CASE list/dashboard copy on `ICSR(s)` wording already requested by the client.
- [x] Dashboard home has already been substantially rebuilt around notices, case counts, quick actions, and appendix-aware behavior.
- [x] Recheck dashboard appendix behavior in UAT because the client reported MFDS visibility issues when appendix selection changes. Run 45 added dashboard coverage proving MFDS appendix counts and configured FDA/MFDS defaults remain visible.
- [x] Confirm how user To Do lists should relate to workflow once WF is finalized. Run 45 verifies the dashboard To Do area stays workflow-facing with draft, QCed, and export/submission shortcuts when workflow is enabled.
- [x] Recheck case-list table-header filtering in the heading cells, not only global filter controls. Run 45 moved CASE list filters into table heading cells and added `case-list-header-filters`.

## P1 Case Section-Specific Recheck

- [x] Recheck CI follow-up-case selector display uses full `C.1.2` timestamp, not date-only, and that source-document uploads show file metadata. CI linked-case options format 14-digit C.1.2 timestamps as `YYYY-MM-DD HH:mm:ss`, and source-document upload stores filename, base64 content, and media type with existing save coverage.
- [x] Recheck RP and SD import-template wording and regional field behavior. Run 45 rechecked existing RP/SD bindings: RP import maps reporter templates into C.2.r fields and SD imports sender templates without receiver/message-header controls.
- [x] Recheck RP `C.2.r.4.KR.1` and reporter FDA email field regional visibility. Regional rendering coverage already verifies FDA/MFDS-only fields by appendix, and reporter email is rendered as FDA-required behavior rather than an ICH-only required field.
- [x] Recheck SD `C.3.1.KR.1` and remove SD receiver/message-header fields if still visible. Existing `field-bindings` coverage verifies Message Header and Receiver Information are not rendered on SD/C.3.
- [x] Recheck LR missing fields and numbering. Run 45 verified LR C.4.r.1/C.4.r.2 rendering and C.4.r.2 included-document upload metadata persistence with `field-bindings`.
- [x] Recheck AE business rules and boolean/null-flavor handling, including `E.i.3.1`, `E.i.3.2`, and `E.i.4`. Existing field-binding and backend validation coverage verifies E.i.4 null-flavor behavior and AE seriousness/highlight fields, with broader business-rule audit tracked separately.
- [x] Recheck LB controlled vocabulary constraints, especially UCUM coverage for `F.r.3.3`. Run 28 converts the test-result unit field from free text to the controlled UCUM option list and verifies backend/frontend UCUM coverage.
- [x] Recheck DG numbering, repeat scopes, WHO-DD behavior, Product ID loading, MFDS conditional requirements, and product/business-rule alignment. Run 28 completed WHO-DD behavior; Run 29 completed Product ID loading from identifier fields. Run 45 verified corrected DG numbering and repeat scopes, G.k.1 drug-not-administered option, free-text G.k.2.2, Product ID visibility, MFDS foreign post-marketing WHO-Drug path, G.k.2.4 country loading, G.k.3.4, G.k.4.r dosage tabs/UCUM-unit controls, and G.k.9.i assessment scope with `field-bindings`.

## P1 Import

- [x] XML/ZIP import, profile selection, and import history are largely implemented.
- [x] Fix `Import Date/Time` so history shows the real import timestamp instead of `Invalid date`. Import history formats valid `uploadedAt` timestamps and falls back to `—` for malformed values, covered by `import-history.date-format`.
- [x] Keep error details downloadable as text if that is the agreed behavior. Import history keeps the `Error text` action wired to `downloadImportHistoryError`, and malformed-date coverage verifies the history view remains usable.

## Mostly Done, Still Verify

- [x] CASE list filtering is broadly implemented but needs a header-filter UX check.
- [x] Case XML bulk export exists but needs operational verification.
- [x] DATA menu access restriction is mostly implemented but should be cross-checked against the final role sheet.
- [x] Separate Audit Logs page removal is mostly done, but the client still expects inline audit trail behavior that must be finished inside CASE/INFO flows.

## Completed Role/Scope Alignment Slices

- [x] Slice 1: Routing and sender visibility. Added role-aware routing profile APIs, sender options, active sender persistence, invalid sender denial, and downstream active-sender case filtering.
- [x] Slice 2: Remaining backend scope enforcement. Added shared case read gates across case lifecycle, validation, import-history error download, patient/narrative lookup, drug subresources, relatedness/recurrence/assessment, and case identifiers.
- [x] Slice 3: Role admin and API shaping. Role list/detail responses now expose canonical role IDs, built-in/editable/sponsor-admin/operational metadata, and structured menu privilege maps. User APIs expose normalized scope and role metadata.
- [x] Slice 4: Frontend implementation. Added a production API-backed static Role and Routing Console in `web-folder/index.html`; Run 08 UAT cleanup fixed final QC terminology, company sponsor-admin sender-scope policy handling, inline script validity, and utilitarian static console styling, with regression coverage in frontend `__tests__/static-role-console.test.ts`.
- [x] Slice 5: Role/privilege cleanup. Added canonical menu-level role privilege normalization, compatibility summary booleans, custom-role edit persistence, unknown-menu rejection, and a frontend privilege-matrix editor for custom roles.

Verification completed for the role/scope slices:
- [x] `cargo fmt --all`
- [x] `cargo check -p web-server --tests --keep-going`
- [x] `cargo test -p web-server scope_visibility_web --test api -- --nocapture`
- [x] `cargo test -p web-server role_admin_api --test api -- --nocapture --test-threads=1`
- [x] `cargo test -p web-server rbac_users --test authz -- --nocapture`

## Implementation Order Suggestion

1. Finish UAT on the implemented role/routing/scope APIs and static console, then close remaining INFO/export/submission visibility gaps found by endpoint testing.
2. Fix case save/QC/lock/follow-up behavior so the core case lifecycle is stable.
3. Re-audit validation, null flavor, regional fields, and repeatable table-style editing.
4. Finish export/submission routing, error handling, and histories.
5. Finish INFO/admin cleanup and final UAT polish.
