# Direct Page Save Transport Unification Design

## Goal

Unify direct case editor page saves behind the same page-projection transport used by `CI` today:

```http
PATCH /api/cases/{case_id}/editor/pages/{section}
```

This change applies to direct sections only: `CI`, `RP`, `SD`, `LR`, `SI`, `DM`, and `NR`. Repeatable row/list sections `DH`, `AE`, `LB`, and `DG` keep their existing row/list save architecture in this phase because their natural save boundary is a row collection, not a single direct page projection.

## Current State

Profile context is request-driven. Editor projections and validation no longer use `cases.appendices_json` or legacy singular `validation_profile`.

The remaining inconsistency is save transport:

- `CI` has both `GET` and `PATCH` page projection endpoints.
- `RP`, `SD`, `LR`, `SI`, `DM`, and `NR` have `GET` page projection endpoints but no `PATCH` page projection endpoint.
- The frontend loads direct route pages through page projection, then saves most sections through older section-specific APIs and refreshes through page projection.
- Repeatable sections still use list and row editor endpoints.

## Architecture

Direct editor pages will use a single request shape:

```json
{
  "profiles": ["fda", "mfds"],
  "changes": {
    "fieldName": {
      "value": "new value",
      "nullFlavor": null
    }
  },
  "rows": {
    "optionalCollectionName": []
  }
}
```

The request body is section-scoped. The editor must not send the full case graph for a direct section save. `changes` is used for scalar field patches such as the existing `CI` implementation. `rows` is used for section collections or section snapshots where the existing persistence model is collection-oriented.

The backend endpoint will:

1. Require case write permissions.
2. Validate `profiles` when present and treat them as the render/validation projection context.
3. Reject unknown fields for the requested section.
4. Apply only the requested direct-section data through existing persistence models.
5. Return the refreshed `CaseEditorPageProjectionResponse`.
6. Echo `profiles`.
7. Never return `focusedAppendix` or case-level selected appendices.

The frontend will:

1. Keep deriving active profiles from route/UI state.
2. Use `PATCH /editor/pages/{section}` for direct page saves.
3. Continue using row/list save APIs for repeatable pages.
4. Preserve the same profile set in the save request and follow-up projection refresh.

## Direct Section Save Boundary

The first implementation phase covers:

- `CI`: existing page patch endpoint remains the baseline.
- `RP`: reporter/primary source data.
- `SD`: sender, receiver, and message header data.
- `LR`: literature data.
- `SI`: study data and study registration rows.
- `DM`: patient, parent, death, and medical history data.
- `NR`: narrative, sender diagnosis, and case summary data.

Each endpoint may internally call existing BMC/update functions. The contract change is at the editor transport boundary, not a database rewrite.

## Repeatable Section Boundary

`DH`, `AE`, `LB`, and `DG` stay on the existing repeatable editor architecture for this phase:

- list endpoint for section overview
- row endpoint for one row
- existing row/subresource saves

This avoids forcing row creation, row deletion, row ordering, and nested child collections into a direct page patch contract before those semantics are designed.

## Validation And Profile Behavior

Save requests must include the current `profiles` whenever the editor is in a direct section route. If omitted, the backend may use the compatibility default already used by projection routes, but frontend direct save code must always send the explicit profile set.

Validation remains explicit:

- page save does not silently run all-profile validation
- validation cache remains keyed by `case_id + profile + page_id`
- list warning counts continue to read cached validation rows
- uncached profile/page combinations display `0` required/warning count until explicit validation populates the cache

## Error Handling

Backend behavior:

- unknown section field: `400 Bad Request`
- invalid profile value: `400 Bad Request`
- missing permissions or locked case: existing authorization/lock errors
- persistence failure: existing service error mapping

Frontend behavior:

- failed page patch keeps the current dirty state visible
- error toast/message uses the section save failure copy already used by the editor
- successful page patch applies the returned projection/readback and keeps the same profile set

## Compatibility

Old section-specific APIs may remain available for non-editor callers during this phase, but direct editor route saves should stop using them once the section has page patch support.

The OpenAPI document must expose page patch endpoints for all direct sections. Client typing must allow `patchEditorPageProjection` for every `DirectEditorSectionCode`, not only `CI`.

## Test Strategy

Backend contract tests:

- every direct section accepts `PATCH /editor/pages/{section}` with explicit `profiles`
- every direct section response includes `profiles`
- no direct section patch response returns case-level `appendices`
- unknown section fields are rejected
- invalid profile is rejected
- existing `CI` behavior remains compatible

Frontend tests:

- direct route save calls `patchEditorPageProjection` with the active page section and explicit profiles
- direct route save no longer calls legacy direct-section save APIs for sections covered by page patch
- save refresh uses the same profile set after a successful page patch
- repeatable sections keep the existing row/list save path

## Out Of Scope

- Converting `DH`, `AE`, `LB`, or `DG` to page patch endpoints.
- Changing validation cache schema.
- Reintroducing case-level appendix metadata.
- Changing export/submission authority behavior.
- Rewriting persistence tables or BMC models.
