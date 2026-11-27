<!-- Historical audit: SQL paths describe that revision. Current schema is db/bootstrap; see db/README.md. -->
# QVIS Safety UI Specification implementation audit

- Source: `QVIS Safety Database_UI Specification_15Jul2026.pdf` (50 pages)
- Audited: 2026-08-03
- Baseline: current working trees, including uncommitted changes in backend and frontend
- Method: PDF visual/text review plus static tracing across frontend, REST handlers, domain models, DB schema, registries, XML import/export, and tests
- Scope note: no production data or authenticated browser session was available; items marked runtime verification require environment reproduction

## Executive result

The PDF is not a clean list of still-open bugs. Several items are implemented in the current code, but major cross-layer defects remain.

Highest-priority root causes:

1. Case scope was not represented as one canonical Sender -> Product -> Study graph. Case visibility, assignment validation, and dependent pickers are now aligned; explicit empty-scope clearing and indirect Sender -> Study validation were added in the follow-up review.
2. Authorization is split between generated actions and hard-coded route/widget behavior.
3. Field actions now persist Notation on the server and Erase through form-model setters; browser-local/DOM mutation paths were removed.
4. Settings are stored but not consistently consumed by runtime UI/export behavior.
5. CIOMS is a custom fixed-coordinate ASCII renderer, causing Unicode, mapping, template, and pagination failures.
6. Import duplicate policy and history status are inconsistent across frontend, domain decision logic, and DB constraints.
7. Several UI catalogs are hand-maintained separately from editor contracts/terminology registries.

## Findings by specification area

### Pages 1-3, 6: login, routing, case visibility, scope hierarchy

| Status | Finding | Root cause and shared fix |
|---|---|---|
| Incorrect | Unassigned Sender is treated as no routing options, while case authorization defines an empty scope as access to all. Evidence: `crates/libs/lib-rest-core/src/lib.rs:400-408,477-490`. | Reuse one `scope_allows` policy for routing options and case visibility. Drive login selection from a routing profile, not role-name branches. |
| Implemented | User create/update now call a shared `validate_scope_assignment()` DB policy; explicit empty arrays clear stored scopes; Product/Study list APIs accept parent scope filters and both create/detail Admin pickers use UUID keys plus dependent selections. Evidence: backend `db/bootstrap/09b-case-scope.sql`, `user_rest/handlers.rs`, `user_rest/validation.rs`, frontend `lib/hooks/usePresaveTemplates.ts`, `app/(protected)/admin/users`, `app/(protected)/admin/user/[userId]`. |
| Incorrect | Imported cases can disappear for Sender-scoped users because case scope reads direct C.3/Product/Study source IDs and ANDs them, while import only populates C.3 source under one setting. Evidence: `lib-rest-core/src/lib.rs:547-590,627-649`, `import_rest.rs:843-878`. | Resolve scope through the canonical Product -> Sender and Study -> Product graph in one shared projection used by lists, history, and authorization. |
| Partial | The canonical projection currently unions direct and derived identifiers. A malformed case with direct Sender S1 plus Product/Study chain Sender S2 can therefore match either scope. | Decide and enforce a source precedence/invariant, or explicitly document union semantics and add a data-quality report for conflicting rows. |

### Pages 4-9, 15: users, roles, permissions, workflow

| Status | Finding | Root cause and shared fix |
|---|---|---|
| Incorrect | User delete/restore still exists although p4 requests removal. Evidence: frontend `admin/users/components/UsersTable.tsx:229-260`; generated `UserDelete` action remains. | If the requirement is final, remove the action/grant at the authorization catalog and close the endpoint as well as hiding UI. |
| Partial | Built-in Sponsor Administrator metadata exists but user detail ignores `isEditable` and leaves role selection enabled. Evidence: backend `user_rest/validation.rs:106-123`; frontend `AdminUserDetailPanel.tsx:106-123`. | Make `roleMeta` the single display/edit contract for user list and detail; retain backend mutation protection. |
| Partial | Role name editing exists, but Description remains disabled and Add dialog terminology is stale. Evidence: `AdminRolesPanel.tsx:187-212`, `RoleCreateDialog.tsx:40-50`. | Use one row draft and explicit save for name/description, gated by built-in metadata. |
| Mostly implemented | Route/sidebar access is action-based, including Import Edit-only access. Evidence: `lib/auth/access-rules.ts:20-112`, `components/Sidebar.tsx:116-121`. | Keep this contract but remove exceptions below. |
| Implemented | Home route/menu and dashboard widgets now consume generated `notice.read`, `notice.update`, and `home.workflow.read` actions; no-action users are not routed to HOME, notice reads do not call runtime settings, and case/workflow widgets do not call case APIs. The backend registry now exposes `home.workflow.read` as a subject action, the catalog hash migration is applied, and the frontend contract is regenerated. Evidence: `routeAccess.ts`, `Sidebar.tsx`, `DashboardPage.tsx`, `NoticePanel.tsx`, `crates/libs/lib-core/src/authorization/registry.rs`, `db/migrations/20260804_authorization_home_workflow_action.sql`. | Keep new action/route/widget matrix tests in CI. |
| Partial | QC and Export/Submission names exist, but My To Do is still Workflow and Report Due Mail is reserved E-mail with no sender/job. Evidence: `generated-authorization.ts:105-113,145-162,205-222,244-263`. | Update the policy registry, regenerate clients, and implement the event/outbox mail path before exposing Send. |
| Partial | Workflow status delete/restore is only client state; save removes the row entirely. Evidence: `useAdminSettings.ts:226-235,264-293`. | Persist stable IDs and tombstones (or active/deleted state) and validate case references by ID. |
| Implemented, runtime check | Two-role workflow save now uses canonical role UUIDs and server validation. Evidence: `adminRolesModel.ts:95-99`, `admin_settings_rest.rs:278-367`. | Reproduce once against the deployed DB. |

### Pages 10, 13, 16: settings and notices

| Status | Finding | Root cause and shared fix |
|---|---|---|
| Partial | Configured timezone affects case creation but audit, Notice, and history use browser locale/timezone. Evidence: backend `case_rest.rs:67-73,941-948`; frontend `lib/audit/auditTrailModel.ts:89-102`, `NoticePanel.tsx:11-15`. | Load runtime timezone once into app context and use one formatter everywhere. |
| Partial | MedDRA setting options are blank when active/approved terminology releases are absent. Evidence: `useAdminSettings.ts:40-83`. | Make terminology-release availability a deployment invariant and return authoritative supported options from settings API. |
| Incorrect | Notation default is stored but CIOMS defaults false and editor/export paths do not resolve it consistently. Evidence: `admin_settings_rest.rs:415`, `cioms_export_rest/build.rs:144-151`, frontend `CaseHeader.tsx:87`. | One resolver: request override -> organization default, shared by XML, CIOMS, and UI initialization. |
| Missing | Three import-date toggles permit every combination, not the four allowed states. Evidence: `ImportDateUpdateSection.tsx:43-57`, `admin_settings_rest.rs:419-424`. | Put the allowed-state machine in the backend contract and generate UI disabled transitions from it. |
| Incorrect | Appendix defaults are stored but navbar uses localStorage/ICH fallback. Evidence: `GlobalNavbar.tsx:15-52,76-100`. | Use runtime settings as the authority context source for selector, route guard, and validation. |
| Incorrect | Notice add/edit replaces the whole array and effective/expire dates are not enforced. Evidence: `NoticePanel.tsx:76-145`, `admin_settings_rest.rs:156-219,518-540`. | Stable per-notice CRUD plus organization-timezone validity filtering. |
| Missing | My To Do dashboard/widget removal is not complete. | Remove the home widget while retaining case workflow permissions separately. |

### Pages 17-21: Sender, Product, Receiver INFO

| Status | Finding | Root cause and shared fix |
|---|---|---|
| Incorrect | Raw numeric Sender Type is displayed instead of label. Evidence: `lib/info/section-contracts.ts:104-117`. | Return/use a shared controlled-terminology `{code,label}` DTO. |
| Partial | Sender deletion dependency checks exist, but the UI does not reliably present structured reasons. Evidence: backend `presave_lifecycle.rs:150-188`; frontend `InfoPresaveListRoute.tsx:96-109`. | Return dependency code/type/count and handle it in a common presave mutation hook. |
| Implemented | Sender audit trigger stores OLD and NEW separately, preserving historical values. Evidence: `db/bootstrap/10-triggers.sql:386-409`. | Existing corrupted historical rows need separate data repair if required. |
| Implemented | Receiver timeline uses one grouped audit button for each day-count/not-applicable field pair. Evidence: `ReceiverForm.tsx`, `PresaveFieldAuditButton.tsx`, `audit_rest.rs`. | Grouped audit subjects query both DB fields and render the changed value in one trail. |
| Incorrect | Product list falls back to relationship UUIDs and backend does not join labels. Evidence: `section-contracts.ts:132-143`, backend `section_presave_rest/product.rs:67-77`. | Return joined `{id,label,deleted}` Sender/Receiver relationships; never display UUID fallback. |
| Incorrect | Product label still says Receiver, and deleted Receiver records remain selectable. Evidence: `ProductForm.tsx:239-271`, backend `receiver.rs:76-87`. | Active-only master-option API by default; historical deleted links shown read-only. Rename to Original Manufacturer. |

### Pages 22-45: Case Editor common behavior

| Status | Finding | Root cause and shared fix |
|---|---|---|
| Implemented | Field Notation is persisted by case/record/field in `case_field_notations`, exposed through the case API, and audited by the generic DB trigger. Erase uses explicit React Hook Form setters; the `localStorage` and DOM event simulation paths were deleted. Runtime verification covered save, reload, delete, and generated CREATE/DELETE audit records. Evidence: `E2BFormField.tsx`, `field-notations.ts`, `case_field_notation.rs`, `case_field_notation_rest.rs`, `20260803_case_field_notations.sql`. | Extend export aggregation only when the specification requires these UI notes in XML/CIOMS output. |
| Implemented | Case editor fields, repeating-table cells, and autocomplete fields now use the shared field-actions path. The autocomplete `...` button previously had no Audit/Erase/Notation behavior. Evidence: `E2BFormField.tsx`, `FormAutocomplete.tsx`; Audit Trail regression suites cover C.1-C.3, D-H, Literature, Study, persisted notation, and model-driven erase. QCed and Locked runtime checks confirmed Audit remains accessible while mutation controls stay disabled. | Keep independent field controls on `FieldActionsButton`; add field-scoped DB targets when a new child table is introduced. |
| Implemented | Field audit identity is normalized through the shared `auditSubject` contract (`tableName`, `fieldPath`, `recordId`). Receiver text and autocomplete fields now query the single `receiver_information` row instead of falling back to recent case history. Evidence: `E2BFormField.tsx`, `FormAutocomplete.tsx`, `ReceiverFields.tsx`, `SectionC3.audit-trail.test.tsx`. | Require `auditSubject` for new independent controls; legacy `auditTable`/`auditField` props remain compatibility inputs. |
| Partial | Repeating-row soft delete/restore is implemented, but deleted case restore is absent. Evidence: `repeatableSoftDelete.ts:16-52`, `RepeatableEditorShell.tsx:151-195`; case delete exists at `case_rest.rs:1343-1389`. | Add symmetric case delete/restore compliance commands with mandatory comments; render Restore for deleted cases. |
| Implemented | QC/Lock read-only behavior, audit availability, unlock, and prior reviewed/validated state restoration are implemented and tested. Evidence: `CaseFormLayout.tsx:76-99`, backend `model/case.rs:767-840`, test `review_lock_web.rs:423-449`. | Runtime-check authority/appendix selector behavior only. |
| Partial | NullFlavor allowed values are centralized and value/NF mutual exclusion works, but the table is manually maintained and UI variants remain mixed. Evidence: `lib/e2b/nullFlavors.ts:12-86`, `NullFlavorButton.tsx:49-217`. | Generate allowed values and renderer variant from editor contracts/registry and use one control. |
| Partial | Country endpoint is complete but UI initially caps display; `EU` is absent from seed. Evidence: `lib/api/countries.ts:6-34`, `db/bootstrap/09a-iso-countries.sql`. | Seed authoritative ISO plus required E2B pseudo-codes and contract-test them. |
| Implemented | UCUM endpoint/frontend mapping retains the full controlled list. Evidence: backend `model/terminology.rs:347-365`, frontend `lib/api/ucum.ts:13-58`. | No separate per-screen UCUM lists. |
| Missing | MedDRA version fields are plain inputs, not release dropdowns. Evidence: `CaseDuplicationCheckPage.tsx:804-815`. | One terminology-release selector shared by all MedDRA version fields/search dialogs. |
| Implemented | FDA Reporter Email, MFDS Other Health Professional Type, FDA/MFDS Study fields, code labels, row-count selector, warning red dot, and basic table soft-delete styling exist. | Preserve shared implementations and test representative pages. |
| Incorrect | Product ID is not required during duplicate/intake creation because `requiredIntakeMatrixFields()` always returns empty. Evidence: `lib/cases/intakeRequiredMatrix.ts:43-47`. | Put mandatory intake fields in one domain contract used by UI and API. |
| Incorrect | Frontend allows confirmed duplicate override while backend policy hard-blocks some duplicate hits. | Define one duplicate decision enum/policy and consume it on both sides after product-policy confirmation. |
| Missing | Narrative template has no separate title; case narrative itself is labeled Template Title, and placeholder buttons remain. Evidence: `components/presave/NarrativeForm.tsx:94-134`. | Separate title from body, remove insertion buttons, retain validated `{element}` parsing/catalog. |
| Partial | Import Template wording and import actions remain inconsistent across pages; some editor pages have no import hook. | One shared presave-import action and page capability contract. |
| Partial | Drug assessment can select AE and repeat/delete/restore, but does not enforce unique reaction selection per drug. | Add domain validation for unique active `reactionId` and filter UI options. |
| Incorrect | MFDS device record is split into scalar fields and separate repeats, while the whole group must repeat. Evidence: `DrugMfdsDeviceInfoFields.tsx:14-145`. | Model `mfdsDeviceInfo[]` as one repeatable record aligned with registry cardinality. |

### Pages 11-12, 46-50: CIOMS, import, export/submission

| Status | Finding | Root cause and shared fix |
|---|---|---|
| Incorrect | CIOMS replaces every non-ASCII character with `?` and uses Helvetica Type1. Evidence: `cioms_export_rest/format.rs:3-13`, `build.rs:88-101`. | Use a Unicode/font-embedding PDF renderer and a Korean glyph render fixture. |
| Incorrect | Portrait output is a scaled landscape custom canvas, not the approved portrait template. Evidence: `types.rs:53-81`, `layout.rs:365-390`. | Make the approved CIOMS template the single source; orientation changes flow/page geometry only. |
| Missing | 7+13 mapping includes only first reaction text and narrative; outcome, drug action, unstructured test data, and additional reactions are omitted. Evidence: `layout.rs:24-30`, `types.rs:163-170`. | Build one explicit mapping aggregator DTO for all five sources and all repeating records. |
| Incorrect | CIOMS output supports at most one continuation page and silently truncates lines/items. Evidence: `build.rs:65-87`, `layout.rs:463-477`. | Cursor-based chunking until EOF with a no-unconsumed-content test. |
| Missing | Submission UI does not pass Notation option; backend notation only combines three narrative fields. Evidence: frontend `lib/api/endpoints/xml.ts:21-64`, `submission/page.tsx:705-786`; backend `canvas.rs:253-293`. | One export-option state plus registry-driven persisted notation collector for XML/CIOMS. |
| Implemented | Import now queues validated XML/ZIP files, exposes active scoped Product IDs from the canonical Product presave list, and sends the selected `productPresaveId` only when the user presses Import. Product selection remains optional so XML-only imports retain their existing behavior. Evidence: frontend `app/(protected)/import/page.tsx`, `lib/api/endpoints/xml.ts`, `__tests__/import-product-picker.test.tsx`; backend `import_rest.rs:107-169,842-878`. | Keep the Product option sourced from the scoped canonical list and retain the explicit-import contract. |
| Incorrect | Duplicate import policy conflates identical-file, case duplicate, and follow-up decisions using partial heuristics. Evidence: `xml_import_decision.rs:46-105`, `import_rest.rs:508-555`. | One domain policy distinguishing file duplicate/case duplicate/follow-up, with explicit review/override contract. |
| Implemented | Import history now uses one `XmlImportHistoryStatus` contract (`success`, `warning`, `skipped`, `error`) across backend writes, API summaries, DB bootstrap/migration, and frontend types. History-write failures now propagate instead of being logged and ignored, so skipped decisions cannot silently disappear. Evidence: `import_rest.rs`, `xml_import_history.rs`, `db/migrations/20260804_import_history_skipped_status.sql`, frontend `lib/types/api.ts` and `app/(protected)/import/page.tsx`. | The separate missing Product picker/explicit Import flow remains to be fixed next. |
| Partial | Export filter operators, range, AND, Last, Check, and post-check actions exist, but the element catalog is a separate partial hand-written list. Evidence: `submission/page.tsx:452-500,895-1166`, backend `case_query_catalog.rs:1-10,269-338,477-514`. | Generate the query catalog from editor contracts/registry/DB mappings. |
| Incorrect | Check/action controls are duplicated and not one bottom-right action footer. Evidence: `submission/page.tsx:895-975`. | One stateful action footer component. |
| Missing | Successful PDF export does not mark or exclude a case from the eligible queue. Evidence: `cioms_export_rest/build.rs:129-173`, `model/case.rs:271-286`. | If confirmed policy, atomically record successful export and filter by export history with an explicit re-export policy. |
| Missing | Report-due mail and automatic Notation translation have storage/permissions only, no sender/worker/provider workflow. | Add idempotent event outbox workers; keep provider boundaries explicit. |

## Recommended implementation order

1. Canonical scope graph and assignment validation (pages 1-3, 6, 17-21, 46) completed; monitor deployed data and conflicting direct/derived source rows.
2. Generated authorization consumption for every menu, route, widget, and API (pages 4-9, 16, 50).
3. Contract-driven field actions and persisted Notation (pages 10, 24, 31-43, 48).
4. Import decision/status contract and explicit Product selection (pages 29, 46-47).
5. CIOMS Unicode/template/mapping/pagination rewrite behind one DTO (pages 11-12).
6. Runtime settings resolver for timezone, appendix, notation, and import dates (pages 10, 13, 16).
7. Registry-generated terminology/query catalogs and remaining cardinality fixes (pages 22-45, 48-49).

## Runtime verification still required

- Reproduce the named `test07125@test.test` account with deployed role/scope data.
- Confirm DB migrations/seeds actually applied, especially terminology releases and country codes.
- Browser-check responsive layout, duplicate icons, authority selector behavior, and exact button placement.
- Render Korean and long CIOMS fixtures and compare pixel/layout output against the approved template.
- Exercise import duplicate/follow-up permutations and verify every outcome appears in history.
- Verify export queue behavior after PDF/XML/line-list completion once product policy is confirmed.
