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

## Source Summary

- `03.csv`: main review backlog by menu/feature with PV comments and follow-up notes
- `roles.csv`: role hierarchy, authority model, and UI/data scope visibility rules
- `wf.csv`: workflow status model, editable-state rules, and sample routing flow
- `list UI.csv`: required line-list/table UI treatment for repeatable sections

## Backlog Rules

- `[ ]` not implemented or still failing UAT
- `[-]` partially implemented and needs recheck
- `[x]` implemented and mainly needs verification only

## P0 Platform, Access, and Naming

- [ ] Make the product/app name configurable and replace the temporary title with the final approved naming.
- [-] Normalize the permission model using `roles.csv` and align it across routing, admin, sender access, QC, lock, and workflow. Role/routing/scope/admin alignment is implemented, including menu-level custom-role privileges and workflow role validation; remaining work is final policy cleanup across QC/lock wording and admin bypass semantics.
- [x] Implement the top-level role hierarchy from `roles.csv`:
- [x] `System Administrator (system-admin)`: can grant/revoke Safety Database access to sponsor administrators but has no in-database working authority
- [x] `Sponsor Administrator (CRO)`: fixed admin role with full read/edit access, role creation, privilege editing, user-role assignment, and sender/product/study/blind scope assignment
- [x] `Sponsor Administrator (Pharmaceutical Company)`: same full admin pattern, but scope excludes sender-level assignment where the client does not expect it
- [x] `User`: permissions derived from assigned role plus assigned work scope
- [x] Preserve sponsor administrator role names/authority as fixed built-in roles if the client expects them to be non-editable defaults.
- [x] Allow sponsor administrators to create additional roles that can match sponsor-admin-level permissions if the client wants equivalent custom roles.
- [x] Make routing page visibility depend on the signed-in user’s role and allowed organization/sender scope.
- [ ] Clarify and fix how QVIS, Client, Organization, and Sender relate so the routing page and admin settings use the same source of truth.
- [-] Apply the UI visibility rules from `roles.csv`:
- [x] CRO sponsor administrators see all senders on the routing page and all related data after routing
- [x] Company sponsor administrators follow the stricter sender rule: no unrestricted sender visibility unless sender scope is explicitly assigned
- [x] normal users see only assigned senders on routing
- [x] CASE shows only allowed product/study values within the routed sender scope
- [-] INFO shows only assigned sender/product/study data. Shared case-linked read gates were added, but INFO list coverage still needs endpoint-by-endpoint UAT.
- [-] IMPORT and EXPORT/SUBMISSION show history only for the user’s allowed product scope. Import-history case-linked error download is gated; remaining history/detail surfaces still need full UAT coverage.
- [ ] Move idle session settings to system-level configuration and confirm whether `Idle Session Limit` and `Warning Lead Time` are the intended admin controls.
- [ ] Standardize system terminology to `QC` / `QCed` instead of mixed review/validated wording.
- [ ] Revisit global menu placement, including where `DATA`, user info, and logout should appear.
- [ ] Decide whether notation auto-translation is required for the first release or should stay in a later phase.

## P0 Case Workflow and Save Model

- [ ] Fix page-level save so it works for both directly entered cases and imported cases.
- [ ] Remove irrelevant batch/header error messages shown during normal case save.
- [ ] Require save/delete reason and comments for compliance-sensitive actions.
- [ ] Remove password re-entry from delete if the client still wants delete confirmation without PW input.
- [ ] Keep deleted cases visible as soft-deleted rows with clear visual marking and history retention.
- [ ] Make case list export history visible from the case area with file, status, error, time, and user.
- [ ] Ensure QC/lock actions behave consistently for manual cases and imported cases.
- [-] Replace ad hoc review state wording with explicit workflow-aware status where the client expects workflow status instead of a generic checked state. Backend workflow status is now distinct from legacy `cases.status`, and the case workflow panel uses the workflow status. Remaining cleanup: broader QC/review wording outside the workflow panel.
- [ ] Implement correct follow-up draft creation from an existing case:
- [ ] Reuse the original case as the source.
- [ ] Set `C.1.1` correctly for the follow-up report.
- [ ] Set `C.1.2` to creation time for the new follow-up.
- [ ] Leave `C.1.5` empty initially.
- [ ] Prefill other fields from the source report.
- [ ] Make `Open Export / Submission` from a case open the target flow with that case already selected and prefilled.

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
- [-] Support status-based role routing so only users in the configured role can act on the case at that workflow step. Implemented for canonical runtime roles and active custom roles, with an intentional safety-db-admin bypass while broader role cleanup remains open.
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

- [-] Re-audit null flavor support across the entire case form, especially required date fields and conditionally mandatory elements. Required-date null-flavor regressions are now covered for sampled C/D/E/F/G date fields, including F.r.1; full-form null-flavor coverage remains open.
- [ ] Re-audit regional element correctness so KR/FDA fields only appear where valid and nonexistent regional fields are removed.
- [ ] Recheck E2B business rules across all case tabs, not just the fields already called out in the CSV.
- [-] Make validation warnings reliable at both section and subsection level so red dots and required indicators match real errors. Backend validation now exposes stable section, subsection, field_path, and section/subsection issue counts; frontend red-dot rendering still needs UAT against the client screens.
- [ ] Replace blocking/non-blocking wording with user-facing terminology the client can understand.
- [ ] Remove `Validation profile` from duplication check and handle appendix selection at the top-level case/home flow instead.
- [-] Ensure date pickers are consistently English, support partial/UK-style requirements where applicable, and block future dates where required. Backend case validation blocks future dates for covered C/D/E/F/G fields; date-picker locale and partial-date UI behavior remain frontend work.
- [ ] Make repeatable structures (`r`, `i`, `k`) use line-list/table-style editing instead of long stacked forms.
- [ ] Implement the line-list UI requirements from `list UI.csv` for repeatable sections not already converted.
- [ ] Make field-level `...` actions support:
- [ ] Audit trail
- [ ] notation where applicable
- [ ] clear/delete value for that field
- [ ] Make audit trail show create/update timestamp, user, field name, value, null flavor, notation, and reason, newest first.
- [ ] Make unsaved-change prompts reliable before navigation away from a case page.
- [ ] Standardize country inputs to ISO 3166-1 alpha-2 list behavior.
- [ ] Finish MedDRA UX:
- [ ] version selection from a list
- [ ] LLT-based search
- [ ] code display with term
- [ ] consistent behavior across all sections
- [ ] Finish WHO-Drug and UCUM data coverage and verify all expected values are present.

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

- [ ] Recheck type-of-report-specific required fields so the matrix is fully applied for spontaneous, study, other, and unknown report types.
- [x] Lock duplicate-create policy: duplicate hits are hard-blocked at create-from-intake, while incomplete basis without a duplicate hit remains explicit-override only.
- [ ] Confirm duplicate detection logic when only part of the duplicate signature matches.
- [ ] Ensure `Product ID` is the loaded value, not a different product label.
- [x] Treat null-flavor placeholders inside duplication check inputs as missing values.

## P0 Appendix and Regional Behavior

- [ ] Move appendix selection to a clear top-level location on the main or case screen.
- [ ] Remove duplicated appendix selection controls when both a direct selector and a dropdown are shown.
- [ ] Ensure MFDS/FDA regional fields render strictly according to the selected appendix combination.
- [ ] Clarify expected XML behavior for multi-appendix cases and document whether output is one XML, multiple XMLs, or authority-specific export paths.

## P0 Submission and Export

- [ ] Separate receiver concepts cleanly:
- [ ] INFO receiver master data
- [ ] export/submission routing receiver configuration
- [ ] Remove receiver identifiers from the wrong INFO location if the client expects them to live only in submission routing configuration.
- [ ] Make export fail gracefully and always record errors in export history with downloadable text details.
- [ ] Finish submission history details:
- [ ] batch result data
- [ ] message result data
- [ ] acknowledged date
- [ ] ACK download
- [ ] ICSR count
- [ ] data file link
- [ ] Add the event column and other requested queue/history columns where still missing.
- [ ] Add search-based case selection for export/submission beyond simple filters:
- [ ] page/section targeting
- [ ] field targeting
- [ ] condition operators
- [ ] value matching
- [ ] Finish the simpler filter set the client explicitly asked for:
- [ ] sender
- [ ] case no
- [ ] QC status
- [ ] lock status
- [ ] workflow status
- [ ] ack accept status if still required
- [ ] Implement Excel line listing export.
- [ ] Recheck imported-case export behavior when sender/receiver/header values are incomplete or mismatched.
- [ ] Confirm authority/report-type-based receiver identifier selection is enforced strongly enough before submission.

## P0 Workflow and Receiver Timeline

- [ ] Make the receiver data load correctly in the Review/RE timeline area.
- [x] Rebuild WF behavior using the client’s separate workflow sheet rather than only the current screen-level status storage.
- [-] Add stronger backend workflow state-transition enforcement if workflow is meant to control edit/QC/lock permissions. Implemented workflow editability, role ownership, assignee ownership, destination validation, assignment-only events, no-op transition rejection, and history persistence. Remaining cleanup: final policy decision on safety-db-admin bypass and deeper QC/lock refactor.

## P0 INFO Master Data

### General

- [-] Align INFO page wording, labels, and record naming with the client’s terminology.
- [ ] Make INFO required markers and notation placement fully consistent with the CASE form pattern.
- [ ] Add table-header filtering for INFO list screens.
- [ ] Make deleted INFO records remain visible as deleted rather than disappearing silently.
- [ ] Make INFO rows open on click for view/edit, matching the client’s line-list expectation.
- [ ] Add field-level audit trail access in INFO screens similar to the requested CASE behavior.

### Sender

- [-] Continue refactoring sender as the operational source of truth for organization/client linkage.
- [ ] Confirm how `Default` should work for sender records.
- [ ] Rework sender-based authorization if the client expects backend-enforced sender ownership across case processing and submission.

### Study

- [ ] Make product selection come from registered master data with the exact product semantics the client expects.
- [ ] Support the requested MFDS and FDA study regional elements completely.
- [ ] Allow the multi-select behavior the client requested where applicable.
- [x] Add `Study Registration (C.5.1.r)` repeatable support.
- [ ] Extend automatic mapping from study/product master data into relevant CASE fields if the client expects this to be automatic.

### Narrative

- [-] Keep the structured narrative fields already added, but remove unwanted `Additional Narrative Fields` content if still present.
- [ ] Decide whether full element-ID-based narrative composition is in scope now or explicitly deferred.

## P0 Admin and User Management

### Roles and Privileges

- [-] Simplify custom role creation so `role_name` and visible name behave the way the client expects. Backend now accepts normalized privilege-based role creation and the frontend admin console exposes a menu-level role editor; final wording/UX still needs UAT.
- [-] Replace `Display name` with `Description` if that matches the requested admin UX. The admin console now uses `Description` in the custom-role flow, but final client label choice still needs confirmation.
- [ ] Clarify and possibly merge `Role Setting` vs `Role & Privilege` if the distinction is confusing to the client.
- [x] Support per-menu permissions for read, edit, QC/review, and lock instead of only coarse role creation.
- [x] Add edit capability for custom roles after creation.
- [-] Reconcile admin UX with the separate client workflow sheet where requested. Workflow status editing is now available in Admin Settings; broader role/privilege UX cleanup remains open.
- [x] Make Safety Database Administrator a role that can be created/managed by sponsor administrators, as described in `roles.csv`.
- [x] Support delegated role creation such as `PVS`, `PVM`, and other client-defined roles from `roles.csv`.

Implemented notes:
- Role admin APIs now normalize either structured menu privileges or legacy boolean inputs into canonical menu-level privilege records.
- Role list/detail responses expose both canonical privilege maps and compatibility summary booleans (`can_view`, `can_review`, `can_lock`, `can_admin`).
- Role privilege validation rejects unknown menu keys and empty privilege sets.
- Frontend admin role editing now uses a per-menu privilege matrix instead of only coarse `View/Review/Lock/Admin` toggles.

### Create / Edit User

- [-] Username removal and organization scoping are mostly done, but the whole create/edit flow still needs UAT recheck.
- [ ] Keep only `Comments` if `Other information` should be removed.
- [x] Fix start/end date handling if account creation still fails when those values are set.
- [x] Allow sender/product/study scope to be edited after user creation.
- [-] Fix role reassignment and user deletion after save. Role reassignment is supported through the normalized update payload; deletion still needs UAT.
- [x] Add the requested blind-flag behavior if required.
- [ ] Enforce the client’s rule that only one admin can hold the relevant admin role, if that is still a hard requirement.
- [-] Improve scope selection UX for large sender/product/study lists, likely with dropdown or popup multi-select. Static console has basic assignment controls; final large-list UX still needs design pass.
- [x] Support scope assignment dimensions from `roles.csv`:
- [x] sender scope
- [x] product scope
- [x] study scope
- [x] blind/non-blind scope where required

### User List and Organization

- [ ] Add table-header filtering to the user list.
- [ ] Clarify the meaning of `Access Window`.
- [ ] Remove any remaining standalone Organization management screen if sender-based organization management is now the intended model.

## P1 Case List and Dashboard Polish

- [ ] Reconfirm whether the client still wants the `ICSRs` wording change or wants different copy.
- [x] Dashboard home has already been substantially rebuilt around notices, case counts, quick actions, and appendix-aware behavior.
- [ ] Recheck dashboard appendix behavior in UAT because the client reported MFDS visibility issues when appendix selection changes.
- [ ] Confirm how user To Do lists should relate to workflow once WF is finalized.

## P1 Case Section-Specific Recheck

- [ ] Recheck CI follow-up-case selector display and attachment behavior.
- [ ] Recheck RP and SD import-template wording and behavior.
- [ ] Recheck LR missing fields and numbering.
- [ ] Recheck AE business rules and boolean/null-flavor handling.
- [ ] Recheck LB controlled vocabulary constraints.
- [ ] Recheck DG numbering, repeat scopes, WHO-DD behavior, and product/business-rule alignment.

## P1 Import

- [x] XML/ZIP import, profile selection, and import history are largely implemented.
- [ ] Fix `Import Date/Time` so history shows the real import timestamp instead of `Invalid date`.
- [ ] Keep error details downloadable as text if that is the agreed behavior.

## Mostly Done, Still Verify

- [x] CASE list filtering is broadly implemented but needs a header-filter UX check.
- [x] Case XML bulk export exists but needs operational verification.
- [x] DATA menu access restriction is mostly implemented but should be cross-checked against the final role sheet.
- [x] Separate Audit Logs page removal is mostly done, but the client still expects inline audit trail behavior that must be finished inside CASE/INFO flows.

## Completed Role/Scope Alignment Slices

- [x] Slice 1: Routing and sender visibility. Added role-aware routing profile APIs, sender options, active sender persistence, invalid sender denial, and downstream active-sender case filtering.
- [x] Slice 2: Remaining backend scope enforcement. Added shared case read gates across case lifecycle, validation, import-history error download, patient/narrative lookup, drug subresources, relatedness/recurrence/assessment, and case identifiers.
- [x] Slice 3: Role admin and API shaping. Role list/detail responses now expose canonical role IDs, built-in/editable/sponsor-admin/operational metadata, and structured menu privilege maps. User APIs expose normalized scope and role metadata.
- [-] Slice 4: Frontend implementation. Added a production API-backed static Role and Routing Console in `web-folder/index.html`; final client visual matching still needs UAT against the screenshots/workflow.
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
