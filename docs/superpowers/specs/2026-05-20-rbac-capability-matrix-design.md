# RBAC Capability Matrix Design

## Purpose

The Role & Privilege admin UI lets administrators assign menu privileges to custom permission profiles. Those privileges must be verified end to end: backend permission mapping, backend API enforcement, frontend navigation visibility, and frontend read-only/write controls must all agree.

The current backend has useful permission-profile tests, but they do not fully prove frontend behavior or every read/write split. The desired architecture is backend-authoritative capabilities exposed to the frontend, with the backend continuing to enforce every API action.

## Architecture

Backend permissions remain enforced in the application layer through `require_permission`, `require_admin`, and `has_permission(ctx.permission_subject(), permission)`. Database RLS remains responsible for data isolation and scope visibility, not feature RBAC.

The backend will expose a normalized capability map on the current-user profile response. The capability map is derived from the same permission subject and permission constants used by the API enforcement path. The frontend will not reimplement permission-profile mapping from role privilege rows.

The current-user profile response will include:

```ts
capabilities: {
  case: { read: boolean; create: boolean; update: boolean; delete: boolean; review: boolean; lock: boolean };
  info: { read: boolean; create: boolean; update: boolean; delete: boolean };
  import: { read: boolean; execute: boolean };
  exportSubmission: { read: boolean; execute: boolean };
  data: { read: boolean; import: boolean; approve: boolean };
  admin: { read: boolean; update: boolean };
  users: { read: boolean; create: boolean; update: boolean; delete: boolean };
  roles: { read: boolean; create: boolean; update: boolean; delete: boolean };
}
```

Existing `roleMeta.canAdmin` may remain as a convenience, but frontend admin access should be derivable from `capabilities.admin.read` or an equivalent backend capability.

## Frontend Behavior

The frontend will add a shared capability accessor, for example `useCapabilities()` and `can(module, action)`.

Navigation:

- A module appears in the sidebar when its read capability is true.
- A module route remains accessible in read-only mode when read is true.
- A module route redirects or shows an access-denied state when read is false.

Controls:

- Create, edit, delete, save, import, export, submit, approve, review, and lock controls require their matching write capability.
- Read-only users still see existing data.
- Inputs, selects, textareas, and mutation controls are disabled or rendered read-only when the user has read but not write permission.
- UI event handlers should also guard before calling mutation APIs, so hidden/disabled controls are not the only protection.

Backend enforcement remains mandatory. Frontend capability checks are for user experience and early prevention only.

## Verification Matrix

For each privilege row exposed by the Role & Privilege UI, tests should prove:

1. The row maps to expected backend permission constants.
2. One representative allowed API succeeds.
3. One representative ungranted API is rejected.
4. Frontend navigation and controls match the capability state.

Initial coverage target:

- `case`: read, edit, review, lock.
- `info`: read and edit.
- `import`: history read and file import.
- `export_submission`: history read and export/submit execution.
- `data`: terminology read, import, and approve.
- `settings` / admin: admin page read and settings update.
- `users`: list/read and create/update/delete.
- `roles`: list/read and create/update/delete.

## Testing Strategy

Backend tests should extend the existing role matrix coverage in `scope_visibility_web.rs` and should keep each privilege row focused on a small set of representative API checks. Tests must verify both positive and negative paths after profile updates, proving `refresh_dynamic_roles()` updates behavior immediately.

Frontend tests should verify capability-driven UI state directly:

- Sidebar item visibility for read/no-read.
- Page access for read-only users.
- Save/create/delete buttons hidden or disabled without write permission.
- Form fields disabled/read-only without write permission.
- Write-capable users see enabled controls.

The frontend should prefer component-level tests around shared gates and representative pages instead of duplicating every backend API matrix test in React.

## Non-Goals

- Do not move feature RBAC into database RLS.
- Do not make custom permission-profile users into built-in admin identities.
- Do not duplicate backend permission mapping logic in frontend code.
- Do not refactor unrelated workflow, audit, or organization-scope behavior.

## Success Criteria

- A custom profile privilege change affects backend API access immediately.
- The current-user profile exposes backend-derived capabilities.
- Read-only roles can view allowed pages but cannot mutate data from the UI.
- Write-capable roles can use the expected controls.
- Tests catch drift between role privilege rows, backend permissions, API behavior, and frontend UI state.
