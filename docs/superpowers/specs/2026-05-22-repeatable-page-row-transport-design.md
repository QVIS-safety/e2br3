# Repeatable Page Row Transport Design

## Goal

Complete save transport unification for repeatable editor sections without losing row precision. Direct pages already use:

```http
GET   /api/cases/{case_id}/editor/pages/{section}
PATCH /api/cases/{case_id}/editor/pages/{section}
```

Repeatable sections will move under the same page namespace, but their write boundary remains one row:

```http
GET    /api/cases/{case_id}/editor/pages/{section}
GET    /api/cases/{case_id}/editor/pages/{section}/rows/{row_id}
POST   /api/cases/{case_id}/editor/pages/{section}/rows
PATCH  /api/cases/{case_id}/editor/pages/{section}/rows/{row_id}
DELETE /api/cases/{case_id}/editor/pages/{section}/rows/{row_id}
```

This applies to `DH`, `AE`, `LB`, and `DG`.

## Scope

Phase 2 adds page namespace list and row endpoints for repeatable sections.

Phase 3 moves the frontend repeatable editor loader and save calls to those endpoints, then removes direct editor route usage from frontend code. The backend may keep old repeatable routes temporarily for compatibility, but frontend editor code should stop depending on them.

## Architecture

Repeatable list projection:

- `GET /editor/pages/{section}` returns `CaseEditorPageProjectionResponse`.
- `pageId` is `DH`, `AE`, `LB`, or `DG`.
- `focusedAppendix` echoes the request appendix when provided.
- `rows` contains the list rows already returned by current list endpoints.
- `fields` stays empty for list projection.
- `sectionSummaries` stays available for future counts but is empty in this phase unless existing code already populates it.

Repeatable row detail:

- `GET /editor/pages/{section}/rows/{row_id}` returns `CaseEditorRowDetailResponse`.
- The payload is the same row detail shape used by the current row endpoint.
- Numeric row positions remain rejected. `row_id` is still a durable UUID.

Repeatable row save:

- `POST /editor/pages/{section}/rows` creates one row and returns row detail.
- `PATCH /editor/pages/{section}/rows/{row_id}` updates one row and returns row detail.
- `DELETE /editor/pages/{section}/rows/{row_id}` deletes one row and returns `204`.
- Requests include explicit `appendix` where the frontend has focused appendix context.
- Save does not run all-profile validation. It marks cached validation stale for the case, matching direct page patch behavior.

## Why Row-Level

Whole-section repeatable patch has too much accidental blast radius: one `AE` save could replace every reaction row, nested assessment, or child list. Row-level page namespace keeps the unified transport while preserving precise row ownership, row delete behavior, and nested save semantics.

## Frontend Behavior

The frontend API gains repeatable page namespace methods:

- `getEditorRepeatablePageProjection(caseId, section, appendix?)`
- `getEditorPageRow(caseId, section, rowId, appendix?)`
- `createEditorPageRow(caseId, section, request)`
- `patchEditorPageRow(caseId, section, rowId, request)`
- `deleteEditorPageRow(caseId, section, rowId, appendix?)`

Route loading changes:

- list routes use `GET /editor/pages/{section}`.
- row routes use `GET /editor/pages/{section}/rows/{row_id}`.
- direct sections keep using existing page projection endpoints.

Save behavior changes:

- repeatable editor row saves call the row page namespace.
- direct editor saves keep using direct page patch.
- full-case wizard saves can keep existing save coordinators until a separate full-wizard save refactor is designed.

## Compatibility

Old repeatable endpoints remain available initially:

```http
GET /api/cases/{case_id}/editor/{section}/list
GET /api/cases/{case_id}/editor/{section}/{row_id}
```

After frontend editor routes stop using them, backend tests should prove the new page namespace covers the editor workflow. Removing old backend routes is a separate compatibility decision after non-editor callers are audited.

## Error Handling

- unknown section: `400 Bad Request`
- unsupported direct/repeatable mismatch: `400 Bad Request`
- numeric row position as row id: `400 Bad Request`
- missing permissions: existing `403` behavior
- locked case: existing case write protection
- missing row: existing `404` behavior

## Testing

Backend contract tests:

- every repeatable section has `GET /editor/pages/{section}`.
- every repeatable section has `GET /editor/pages/{section}/rows/{row_id}`.
- row position ids are rejected under the new namespace.
- `POST/PATCH/DELETE` row endpoints persist and remove one row.
- responses do not return case-level selected appendices.

Frontend tests:

- API client calls the new repeatable page namespace.
- route loading uses new repeatable list and row endpoints.
- repeatable row save uses page row patch/create/delete methods.
- direct page save behavior remains unchanged.
- legacy frontend calls to `/editor/{section}/list` and `/editor/{section}/{row_id}` are removed from editor route code.

## Out Of Scope

- Removing backend legacy repeatable routes in the same pass.
- Converting full-case wizard save to row page namespace.
- Running validation automatically on row save.
- Changing validation cache schema.
- Reintroducing case-level appendix metadata.
