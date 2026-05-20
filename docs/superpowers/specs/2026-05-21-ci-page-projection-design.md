# CI Page Projection and Patch Save Design

## Context

The reference cubeSAFETY CI page uses a page-oriented edit model. Loading the CI page calls page, version, report, code-list, and `pages/CI` endpoints. Saving a CI edit is followed by a saved-status check and a refreshed CI page/status reload. The refreshed page model carries field-level value metadata, origin values, warnings, `empty`, `requiredEmpty`, and page-level required counts.

SafetyDB currently has a cleaner centralized validation engine, but editor persistence and validation projection are split. CI save is section-scoped through frontend coordinators and backend subresource endpoints, yet a dirty safety-report section generally sends a section-shaped payload rather than an explicit field patch. Validation is then fetched separately through `/api/cases/{case_id}/validation` or `/api/cases/{case_id}/validation/all` and mapped back onto frontend fields.

The target is a production-grade CI pilot that keeps centralized validation while adding a precise page projection layer for editing.

## Goals

- Make CI editing precise and section-scoped.
- Save only intentional changes with explicit clear/null semantics.
- Return a server-owned CI page model after read and save.
- Project appendix-specific validation into field-level editor state.
- Keep existing validation rules centralized in the backend.
- Preserve existing subresource APIs during migration.
- Establish a pattern that can later migrate RP, SD, SI, DM, AE, DG, and other sections.

## Non-Goals

- Migrating every case edit section in the first slice.
- Replacing the canonical validation report endpoints.
- Moving business-rule validation into the frontend.
- Blocking ordinary draft save on required-field validation issues.
- Reworking submission/export validation semantics.

## Recommended Architecture

Add a CI page projection API over the existing domain models and validation engine:

- `GET /api/cases/{case_id}/editor/pages/CI`
- `PATCH /api/cases/{case_id}/editor/pages/CI`

The page service owns CI editor composition. It loads existing domain data, resolves appendix context, projects validation issues onto fields and rows, and returns a UI-ready page model. The same projection shape is returned after GET and PATCH so the frontend can reset from server truth after every save.

The existing safety report, message header, linked report, document, and other identifier endpoints remain available for compatibility while the CI editor switches to the new page API.

## Page Read Flow

1. Authorize case read and section read permissions.
2. Load case shell and selected `appendices_json`.
3. Load CI-owned domain data:
   - safety report identification,
   - message header fields relevant to CI,
   - documents held by sender,
   - other case identifiers,
   - linked report numbers.
4. Resolve appendix context from selected appendices and optional focused appendix.
5. Run centralized validation for selected appendices.
6. Project validation issues onto fields and repeatable rows.
7. Return page-level status, required counts, field envelopes, and row envelopes.

## Page Save Flow

1. Authorize case update and section update permissions.
2. Reject locked or non-editable workflow states through existing write guards.
3. Validate patch syntax and field ownership.
4. Apply only changed scalar fields.
5. Apply repeatable row operations using stable row UUIDs where available.
6. Preserve explicit clear/null semantics instead of relying on empty strings.
7. Reload CI domain data.
8. Re-run appendix-aware validation projection.
9. Return the refreshed CI page projection.

Validation issues should not block ordinary save unless the issue is a persistence invariant, such as an invalid field type, unknown code value for a constrained storage column, missing row identity for an update/delete, or a permission/workflow failure.

## Patch Request Contract

The request is field-patch based, not whole-case and not full-section payload.

```json
{
  "appendix": "fda",
  "changes": {
    "reportType": { "value": "2" },
    "localCriteriaReportType": { "value": "1" }
  },
  "rows": {
    "documentsHeldBySender": {
      "upsert": [],
      "delete": []
    },
    "otherCaseIdentifiers": {
      "upsert": [],
      "delete": []
    },
    "linkedReports": {
      "upsert": [],
      "delete": []
    }
  }
}
```

Field patch values use explicit semantics:

- omitted field: no change
- `{ "value": "2" }`: set value
- `{ "value": null }`: clear value
- `{ "nullFlavor": "UNK" }`: set null flavor and clear value where the field supports null flavor
- `{ "value": "20260520", "nullFlavor": null }`: set value and clear null flavor

Unknown fields, fields not owned by CI, and values invalid for their storage type are request errors.

## Page Projection Response

Each scalar field is returned as a field envelope:

```json
{
  "fieldId": "CASE_RPT_TYPE",
  "path": "safetyReportIdentification.reportType",
  "label": "Type of Report",
  "value": "2",
  "display": "Report from study",
  "nullFlavor": null,
  "notation": null,
  "originValue": "2",
  "originNullFlavor": null,
  "visible": true,
  "editable": true,
  "empty": false,
  "requiredEmpty": false,
  "issues": []
}
```

The page envelope includes:

```json
{
  "caseId": "uuid",
  "pageId": "CI",
  "appendices": ["ich", "fda"],
  "focusedAppendix": "fda",
  "saved": true,
  "requiredCount": 2,
  "fields": {},
  "rows": {},
  "sectionSummaries": []
}
```

Repeatable rows include stable row identity, display sequence, deleted state when applicable, saved state, origin values, and field envelopes for row-owned values.

## Appendix and Visibility Model

Appendix context is backend-owned. Each projected field has visibility metadata derived from a registry:

- base ICH fields are visible for all profiles.
- FDA fields are visible when FDA is selected or focused.
- MFDS fields are visible when MFDS is selected or focused.
- future EU, PMDA, or NMPA fields can be added without changing the projection contract.

The validation engine can still compute full reports for all selected appendices. The page projection filters displayed issues to the fields relevant to the active editor context. Hidden fields may remain persisted, but hidden appendix fields should not create visible editor warnings unless they are required for the active appendix context.

## Validation Projection

The existing `CaseValidationReport` and `ValidationIssue` remain canonical. The page projection layer maps issues to fields by:

1. resolving `field_path` or `path`,
2. mapping the path to a CI field or row field,
3. filtering by visibility and appendix context,
4. setting field `issues`,
5. setting `requiredEmpty` for missing required field issues,
6. incrementing page and subsection required counts.

This avoids duplicating business rules inside page serializers or frontend components.

## Frontend Integration

The CI page should switch from composing data through the current save coordinator to consuming the projection API.

Read:

- call `GET /editor/pages/CI`,
- render field envelopes directly,
- render code labels from projected display values and field metadata,
- use returned required counts for CI tab state.

Save:

- compute a patch from dirty field envelopes,
- send `PATCH /editor/pages/CI`,
- replace local CI state with the returned projection,
- reset dirty baseline from the returned origin/current values.

The existing frontend validation-banner mapping remains available for non-migrated sections until each section gets a projection API.

## Error Handling

Patch request errors return 4xx with a structured error code and field path:

- unknown field,
- field not owned by CI,
- invalid field type,
- invalid code value,
- invalid row operation,
- missing row identity,
- locked or unauthorized case.

Persistence failures return the existing API error shape with enough context for the frontend to show a section save failure.

Validation findings return inside the page projection as field or row issues and do not make the save fail unless they are persistence invariants.

## Migration Plan

The first slice is CI only.

1. Add backend projection DTOs and CI field registry.
2. Add backend GET projection endpoint.
3. Add backend PATCH endpoint for scalar CI fields.
4. Add repeatable row patch operations for CI child rows.
5. Switch frontend CI loading behind the existing section-scoped route.
6. Switch frontend CI save to patch API.
7. Keep existing subresource endpoints for compatibility.
8. Expand the same model section by section after CI proves stable.

## Testing Strategy

Backend tests:

- GET CI projection returns field envelopes and page required counts.
- ICH-only projection hides FDA-only fields and FDA-only warnings.
- FDA projection shows FDA fields and FDA required warnings.
- PATCH C.1.3 changes only report type.
- PATCH explicit null clears a nullable field.
- unknown or non-CI field patch is rejected.
- repeatable row upsert/delete preserves UUID identity.
- save response returns refreshed warnings and required counts.

Frontend tests:

- CI renders projected field envelopes.
- dirty C.1.3 sends a field patch, not a whole case or full section payload.
- returned projection resets the saved baseline.
- server-returned field issues render next to fields.
- appendix switch changes visible fields and warnings.
- required count updates after save.

Integration/UAT:

- edit C.1.3 and save,
- verify only CI page API is used for the migrated path,
- verify FDA required warnings match page projection,
- verify existing validation report endpoint still returns canonical reports,
- verify non-CI sections still work through existing paths.

## Risks and Mitigations

- Risk: field registry drift from validation paths.
  Mitigation: add tests that every CI validation path maps to a known projected field or an explicit page-level issue.

- Risk: hidden appendix fields create confusing warnings.
  Mitigation: projection filters issues through visibility context before rendering field state.

- Risk: PATCH semantics differ from existing `COALESCE` update behavior.
  Mitigation: introduce explicit patch-value handling at the page service boundary and add clear/null tests.

- Risk: migration duplicates save logic temporarily.
  Mitigation: keep the page service as an orchestration layer over existing BMC methods until shared lower-level helpers are warranted.

## Success Criteria

- CI can be loaded and saved through page projection APIs.
- A C.1.3 edit persists through a precise field patch.
- Server response after save includes field-level validation state and page required count.
- FDA/ICH appendix differences are handled by backend visibility and validation projection.
- Existing validation report endpoints and non-CI section saves remain stable.
- Tests prove request precision, appendix filtering, warning projection, and saved baseline reset.
