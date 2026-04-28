# Workflow Rebuild Plan

Date: 2026-04-13

This plan covers the workflow rebuild required by [wf.csv](/Users/hyundonghoon/projects/rust/e2br3/e2br3/docs/requirements/wf.csv) without taking on the role-model redesign yet.

## Current State

The backend currently uses `cases.status` as a mixed-purpose field for:

- draft/save state
- QC/review state
- lock state
- submission lifecycle state

There is also a `workflow_routes_json` field, but it is just stored as freeform JSON and is not enforced as workflow state.

This does not match the client workflow requirement, which says:

- every case has exactly one workflow status
- workflow status is separate from QC and Lock
- workflow is configurable from Admin
- workflow is off by default
- only configured editable statuses allow case editing
- routing requires role, user, comments, and optional due date

## Recommendation

Do not replace `cases.status` immediately.

Instead, introduce a separate workflow model alongside the existing case lifecycle/QC/lock/submission status model. This keeps export, validation, and existing read-only behavior from breaking while we rebuild workflow properly.

## Target Data Model

### 1. Keep existing `cases.status`

Keep `cases.status` for the existing legacy lifecycle until QC/lock are refactored cleanly.

Short-term meaning:

- `validated` remains the export/submission gate
- `locked` remains the current lock gate
- legacy `reviewed` behavior can be retired after workflow status is live

### 2. Add explicit workflow columns to `cases`

Add these columns:

- `workflow_status text not null default 'Saved'`
- `workflow_assigned_role text null`
- `workflow_assigned_user_id uuid null references users(id) on delete set null`
- `workflow_due_at timestamptz null`
- `workflow_description text null`
- `workflow_updated_at timestamptz not null default now()`

Why columns instead of another JSON blob:

- simpler filtering on case list and submission/export queue
- easier enforcement in shared write guards
- easier API contracts and tests
- matches the client requirement that workflow status is first-class case data

### 3. Add workflow route/event history table

Add a new table:

`case_workflow_events`

Suggested columns:

- `id uuid primary key default gen_random_uuid()`
- `case_id uuid not null references cases(id) on delete cascade`
- `from_status text not null`
- `to_status text not null`
- `target_role text null`
- `target_user_id uuid null references users(id) on delete set null`
- `comment text null`
- `due_at timestamptz null`
- `acted_by uuid not null references users(id)`
- `created_at timestamptz not null default now()`

This table is the backend source for:

- route history
- comments on handoff
- target user history
- due date history

### 4. Store workflow configuration in `app_settings`

Do not create workflow master tables in the first pass.

Use `app_settings.key = 'system'` and extend the JSON payload with a `workflow` object:

```json
{
  "workflow_enabled": false,
  "workflow": {
    "statuses": [
      {
        "name": "Saved",
        "editable": true,
        "description": "Default authoring state",
        "allowed_roles": ["PVS", "PVM", "Safety Database Administrator"]
      },
      {
        "name": "To be reviewed",
        "editable": false,
        "description": "Pending internal review",
        "allowed_roles": ["PVS", "Safety Database Administrator"]
      },
      {
        "name": "Internal review completed",
        "editable": false,
        "description": "Reviewed and routed back",
        "allowed_roles": ["PVS", "PVM", "Safety Database Administrator"]
      },
      {
        "name": "Finalized",
        "editable": false,
        "description": "Final workflow state",
        "allowed_roles": ["Safety Database Administrator"]
      }
    ]
  }
}
```

Why `app_settings` first:

- the repo already uses it for system configuration
- this avoids adding admin CRUD tables before the workflow contract exists
- the config can later be normalized into dedicated tables without breaking case-level workflow data

## API Shape

## Admin Settings

Extend [admin_settings_rest.rs](/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/src/web/rest/admin_settings_rest.rs) with:

- `workflow_enabled: false` as the default
- `workflow: Option<WorkflowConfigPayload>`

Add typed payloads:

- `WorkflowConfigPayload`
- `WorkflowStatusConfigPayload`

Validation rules:

- workflow may be omitted
- if workflow is enabled and no statuses are configured, inject default `Saved`
- status names must be unique
- at least one status must be editable
- `Saved` must exist unless the client explicitly approves a different baseline

## Case APIs

Extend case create/get/update docs and payloads with:

- `workflow_status`
- `workflow_assigned_role`
- `workflow_assigned_user_id`
- `workflow_due_at`
- `workflow_description`

Do not let normal case updates arbitrarily overwrite workflow fields.

Add a dedicated transition endpoint:

- `POST /api/cases/:id/workflow/transition`

Suggested request shape:

```json
{
  "data": {
    "to_status": "To be reviewed",
    "target_role": "PVM",
    "target_user_id": "uuid",
    "comment": "Ready for internal review",
    "due_at": "2026-04-20T09:00:00Z"
  }
}
```

Suggested response:

- updated case workflow fields
- the created workflow event row

Why a dedicated endpoint:

- transition validation is separate from ordinary field updates
- route comments/history become mandatory where needed
- tests become much cleaner than overloading `PUT /api/cases/:id`

## Enforcement Rules

### Shared write guard

Replace the current `reviewed/locked`-only workflow logic in [lib.rs](/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/libs/lib-rest-core/src/lib.rs) with:

1. check whether workflow is enabled
2. if enabled, load workflow config from `app_settings`
3. find the case's `workflow_status`
4. deny case-content writes when that status is not editable
5. continue to deny writes when legacy `status = locked`

Short-term rule:

- `locked` still blocks writes
- workflow status controls normal editability
- legacy `reviewed` should stop being the write gate once workflow editability is active

### Transition enforcement

On workflow transition:

1. verify workflow is enabled
2. verify destination status exists in workflow config
3. verify current user is allowed for the current workflow step
4. verify target role/user fields as required by the client flow
5. update case workflow columns
6. insert `case_workflow_events` row

## Migration Sequence

### Phase 1: introduce workflow state without breaking current flows

1. add new case workflow columns
2. add `case_workflow_events`
3. extend admin settings payload with typed workflow config
4. default `workflow_enabled` to `false`
5. backfill all existing cases with `workflow_status = 'Saved'`

### Phase 2: wire enforcement

1. add workflow config loader/helper
2. add dedicated transition endpoint
3. switch shared write guard from `reviewed` to workflow editability
4. keep `locked` as a hard stop

### Phase 3: remove legacy review coupling

1. stop using `reviewed` as workflow shorthand
2. remove `reviewed` read-only semantics
3. keep `validated` and `submitted` behavior for export/submission until QC/lock redesign is done

## Tests To Add

Add integration tests for:

- default settings return `workflow_enabled = false`
- existing cases default to `workflow_status = Saved`
- editable status allows content writes
- non-editable status blocks content writes
- transition inserts route history row
- invalid destination status is rejected
- workflow-disabled mode ignores workflow transition endpoint
- case list filtering supports workflow status

## Files Likely To Change

- [01-safetydb-schema.sql](/Users/hyundonghoon/projects/rust/e2br3/e2br3/db/bootstrap/01-safetydb-schema.sql)
- [case.rs](/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/libs/lib-core/src/model/case.rs)
- [admin_settings_rest.rs](/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/src/web/rest/admin_settings_rest.rs)
- [case_rest.rs](/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/src/web/rest/case_rest.rs)
- [lib.rs](/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/libs/lib-rest-core/src/lib.rs)
- [openapi.rs](/Users/hyundonghoon/projects/rust/e2br3/e2br3/crates/services/web-server/src/openapi.rs)
- new workflow transition tests in `crates/services/web-server/tests/`

## First Implementation Slice

The safest first code slice is:

1. add workflow columns and `case_workflow_events`
2. change admin settings default `workflow_enabled` from `true` to `false`
3. add typed workflow config to admin settings
4. expose workflow fields on case GET/list
5. do not enforce editability yet

That gives us the correct data model first, with minimal blast radius.
