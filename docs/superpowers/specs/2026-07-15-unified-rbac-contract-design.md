# Unified RBAC Contract Design

## Goal

Make backend authorization and frontend affordances use one permission model so that, for the same user and policy version, every action shown as available by the frontend passes the backend authorization check and every action allowed by the backend is reachable from the frontend.

This is an architectural correction, not a collection of per-screen 403 fixes.

## Confirmed Root Cause

The backend authorizes REST operations with granular `Permission(Resource, Action)` values. The frontend independently derives access through four different mechanisms:

- coarse module `capabilities`;
- `roleMeta` summary booleans;
- direct role-name comparisons;
- workflow or record state without checking the user's permission.

These representations lose information and make different decisions. The mismatch is reproducible across INFO, CASE, IMPORT, SUBMISSION, and DATA:

- INFO read-only users see create, update, and delete controls whose APIs return 403;
- CASE write controls are governed by record lifecycle state rather than user update permissions;
- IMPORT sidebar and route rules disagree for execute-only users;
- SUBMISSION read-only users see export and submit actions whose APIs return 403;
- dynamic DATA permissions pass backend terminology checks while the frontend rejects non-system-admin role names.

Backend effective-access tests and frontend authorization/UI tests both pass independently, demonstrating that the two test suites currently validate different contracts.

## Design Principles

1. The backend `Permission(Resource, Action)` catalog is the sole authorization vocabulary.
2. Roles and menu privilege matrices produce permission sets; they are not alternative authorization mechanisms.
3. Backend route guards and frontend routes, menus, controls, and actions reference the same permission identifiers.
4. Page entry and operations within a page are separate permission decisions.
5. The backend remains the final enforcement boundary; frontend checks prevent misleading affordances.
6. Permission changes must invalidate stale frontend authorization state.
7. Frontend and backend are deployed together. No legacy `capabilities` fallback or compatibility mock is required.

## Backend Permission Contract

`GET /api/users/me/profile` returns the effective permission set and policy version:

```json
{
  "user": {
    "id": "...",
    "role": "qa_reviewer"
  },
  "permissions": [
    "Case.List",
    "Case.Read",
    "Case.Update",
    "SafetyReport.Read"
  ],
  "policyVersion": 42,
  "activeOrganization": {},
  "availableOrganizations": [],
  "routing": {}
}
```

The permission list is the final effective set after resolving built-in or dynamic roles. It is sorted and deduplicated for deterministic responses and tests.

The permission identifier format is the existing `Resource.Action` display format already used by `PermissionDenied.required_permission`.

The response no longer contains `capabilities`. Capability DTOs, aggregation functions, OpenAPI schema fragments, and capability-specific tests are removed in the same deployment.

`policyVersion` changes whenever an authorization policy that can affect effective permissions changes. The initial implementation uses a database-backed monotonic version and refreshes the process permission cache in the same administrative mutation. This keeps the value meaningful across restarts and prepares the service for multiple instances.

## Shared Permission Catalog

The Rust permission catalog generates a committed TypeScript artifact containing:

- the complete string-literal union;
- named constants for every permission;
- the catalog version or generation marker.

Frontend code must import these generated identifiers rather than writing permission strings manually. CI regenerates the artifact and fails on a diff, preventing stale or invented frontend permissions.

The generator is one-way: Rust remains the source. The generated artifact contains identifiers only and does not duplicate role or menu policy logic.

## Frontend Authorization Boundary

All authorization helpers live behind one `lib/auth/permissions` boundary. The public API is limited to:

```ts
can(permission)
canAny(permissions)
canAll(permissions)
<PermissionGate require={...} mode="all|any">
```

The authenticated profile's permission array is converted once into a read-only set. Callers do not inspect the array or user role directly.

The following are prohibited as authorization inputs:

- `role === "system_admin"` or other role-name checks;
- `roleMeta.canAdmin` or equivalent summary fields;
- module capability booleans;
- record lifecycle state without a permission check.

Role names remain valid business data for display, workflow assignment, and other cases where the role itself—not authorization—is the domain value.

Route and menu requirements are declared as data using the same permission constants. A route may define an `anyOf` entry requirement while individual controls define their precise action permission.

## Module Rules

### INFO

Each Sender, Receiver, Product, Reporter, Study, and Narrative list, detail, create, update, delete, restore, and audit action declares the exact permission required by its backend endpoint. A general INFO page does not infer write access from read access or from a different INFO resource.

### CASE

Case list and shell access use their exact read/list permissions. Each editor section declares all permissions required by its load and save endpoints. Save is enabled only when the user has the section's update permissions and the record lifecycle permits editing.

Review and lock operations require `Case.Approve`; normal edits require `Case.Update` plus endpoint-specific resource permissions. Lifecycle state remains a business constraint, not an authorization substitute.

### IMPORT

History and error downloads require `XmlImport.Read`. Upload and execution require `XmlImport.Import`. Route entry may allow either permission, while the upload and history areas are gated independently.

### SUBMISSION AND EXPORT

History requires `XmlExport.Read`. Creating exports, submitting cases, and other execution operations require `XmlExport.Export`. Read access never exposes execution controls.

### DATA

Terminology views and actions use `Terminology.Read`, `Terminology.Import`, and `Terminology.Approve`. Dynamic roles work identically to built-in roles. The frontend does not impose an additional system-admin role-name restriction.

### ADMINISTRATION

Users, permission profiles, settings, audit logs, notices, and organizations use the exact permissions enforced by their REST handlers. Any operation that is intentionally system-admin-only is represented by an explicit permission granted only to the built-in system-admin role, rather than a parallel role-name check.

## Policy Synchronization

Every authenticated API response includes `X-RBAC-Policy-Version`. The frontend API client compares it with the loaded profile version.

When a mismatch is observed:

1. one deduplicated profile refresh begins;
2. new mutating interactions are temporarily disabled;
3. the permission set is atomically replaced;
4. routes and controls are recomputed;
5. the original backend response is still handled normally and is not silently retried.

The frontend also verifies the profile on protected route transitions and browser focus return. This closes the common stale-tab path without treating 403 as routine UI control.

## Permission-Denied Contract

Permission denials use one stable payload:

```json
{
  "error": {
    "code": "PERMISSION_DENIED",
    "requiredPermission": "StudyRegistration.Update",
    "policyVersion": 42,
    "requestId": "..."
  }
}
```

The frontend first applies policy-version synchronization when needed, then displays a consistent denial message. Development diagnostics record the attempted action, the frontend permission decision, the required backend permission, policy version, and request ID. Sensitive user or payload data is excluded.

## Testing Strategy

Implementation starts from failing cross-layer contract tests derived from the confirmed reproductions.

Backend coverage includes:

- complete permission catalog serialization;
- built-in and dynamic role effective permission sets;
- profile `permissions` and `policyVersion` contract;
- permission-profile mutation and version increments;
- every REST endpoint's required permission;
- stable permission-denied payloads.

Frontend coverage includes:

- generated catalog freshness;
- permission helper truth tables;
- route and sidebar decisions from permission fixtures;
- module-level control visibility and enabled state;
- CASE composite read/write requirements;
- policy-version mismatch synchronization and request deduplication;
- removal of capability and authorization role-name usage.

A cross-layer manifest test compares each frontend action declaration with the backend endpoint permission contract. This catches both directions of drift: UI actions missing a backend permission and backend operations unreachable from the UI.

No production fallback capability model or mock authorization path is introduced. Tests use concrete permission identifiers from the generated catalog.

## Migration Sequence

1. Add failing cross-layer reproduction tests and endpoint/action inventory.
2. Expose deterministic effective permissions and policy version from the backend.
3. Generate the TypeScript permission catalog and add freshness enforcement.
4. Introduce the single frontend authorization boundary.
5. Migrate route, sidebar, and module actions in bounded slices while keeping tests red-to-green per slice.
6. Add policy synchronization and stable denial diagnostics.
7. Remove capabilities, role-summary authorization, and role-name authorization checks.
8. Run full backend, frontend, generated-contract, and cross-layer suites.

Because frontend and backend deploy together, the legacy contract is removed only after every consumer has migrated, then both repositories are released as one coordinated change.

## Success Criteria

- For the same user and policy version, every frontend-enabled action passes backend permission enforcement.
- Every backend-allowed user operation represented in the product is reachable in the frontend.
- No frontend authorization decision uses capabilities, role metadata summaries, or direct role-name comparison.
- Dynamic and built-in roles follow the same frontend and backend paths.
- Permission policy changes converge without requiring logout.
- Contract drift fails automated tests before deployment.

## Out of Scope

- Redesigning workflow role assignment semantics.
- Changing organization, sender, product, study, or blind-data scope rules.
- Replacing backend authorization enforcement with frontend checks.
- Supporting mixed old/new frontend and backend deployments.
