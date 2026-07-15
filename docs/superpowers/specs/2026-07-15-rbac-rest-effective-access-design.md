# RBAC REST Effective Access Tests Design

## Goal

Verify that persisted dynamic permission profiles control real HTTP endpoints,
not only in-memory permission checks or capability responses. Tests cover the
complete path from profile persistence and user assignment through Router
authorization and HTTP status mapping.

## Existing Coverage and Scope

Existing `tests/authz` files already cover built-in administrator/viewer access
for cases, patients, drugs, narratives, safety reports, users, organizations,
and audit endpoints. The new suite does not repeat those built-in-role tests.

The new suite focuses on dynamic-profile effective access for functional areas
that are missing or only partially represented: case operations, information
and presave endpoints, import/export, terminology, administrative settings,
and dashboard notices. Production code is unchanged unless a test exposes a
confirmed authorization defect.

## File Structure

Add a focused module folder below the existing role-admin API integration tests:

```text
crates/services/web-server/tests/api/role_admin/effective_access/
├── support.rs
├── case_web.rs
├── information_web.rs
├── transfer_web.rs
├── terminology_web.rs
├── administration_web.rs
└── dashboard_web.rs
```

`tests/api/role_admin/effective_access.rs` is the module entry point, and
`tests/api/role_admin/mod.rs` includes it.

## Shared Test Flow

`support.rs` builds on existing role-admin helpers. Each test:

1. initializes the PostgreSQL test model manager and organization fixtures;
2. creates an empty permission profile through the REST API;
3. creates a user assigned to the generated profile and obtains its cookie;
4. calls a protected Router endpoint and verifies HTTP 403 with the expected
   permission-denied error contract;
5. updates the persisted profile privileges through the REST API;
6. repeats the request with the same user and verifies the endpoint is no longer
   forbidden, using the endpoint's specific success status where fixtures make
   that deterministic;
7. calls an unrelated protected endpoint and verifies it remains forbidden.

Tests use `#[serial]` because database fixtures and the process-global dynamic
role registry are shared. Unique names and identifiers prevent row collisions.

## Area Coverage

- `case_web.rs`: case read/list and edit/write access, including a denied write
  for a read-only profile.
- `information_web.rs`: representative presave/information read and write
  endpoints, with read-only versus edit separation.
- `transfer_web.rs`: import read/execute and export read/execute endpoints,
  including submission aliases where they map to distinct routes.
- `terminology_web.rs`: terminology read, import, and approve endpoint gates.
- `administration_web.rs`: settings read/update and user read/write gates; roles
  and system-only organization behavior remain explicitly separated.
- `dashboard_web.rs`: notice read/update endpoints and confirmation that the
  e-mail permission does not grant unrelated settings access.

Each file contains focused named tests rather than one route matrix. Endpoint
fixtures may return domain validation errors after authorization; helpers
distinguish those from 403 so the test proves the permission gate was crossed.

## Assertions

Denied responses must be HTTP 403. Where the JSON error body is available, it
must include the permission-denied detail contract. Granted requests must use a
specific success status when the test supplies valid fixtures; otherwise they
must assert `status != 403` and document the downstream validation status being
accepted.

Every granted profile test also checks at least one unrelated endpoint remains
403, preventing broad accidental privilege expansion.

## Verification

Run the new focused integration module, the existing `authz` integration target,
the complete role-admin API target, and the full `web-server` test suite. Tests
that require unavailable external services are not added; PostgreSQL is the
only required integration dependency already used by the project.
