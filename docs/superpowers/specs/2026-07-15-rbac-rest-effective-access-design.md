# RBAC REST Effective Access Test Split Design

## Goal

Split the existing 1,600-line dynamic-profile REST test module into focused
area files without duplicating tests or changing their database and Router
behavior. Add only the missing authorization assertions exposed during the
split.

## Existing Coverage

`tests/api/role_admin/privilege_matrix_web.rs` already exercises persisted
permission profiles for case, information/presave, terminology, export,
import, users/roles, settings, workflow, audit, notice, and e-mail behavior.
Those tests create profiles through HTTP, assign users, update privileges, and
verify effective permissions or Router responses.

The implementation therefore moves this coverage instead of recreating it.

## Target Structure

```text
crates/services/web-server/tests/api/role_admin/
├── effective_access.rs
└── effective_access/
    ├── support.rs
    ├── persistence_web.rs
    ├── case_web.rs
    ├── information_web.rs
    ├── transfer_web.rs
    ├── terminology_web.rs
    ├── administration_web.rs
    └── dashboard_web.rs
```

`role_admin/mod.rs` replaces `mod privilege_matrix_web` with
`mod effective_access`. The old `privilege_matrix_web.rs` is deleted after all
tests have been moved.

## File Responsibilities

- `support.rs`: imports and helper functions shared only by effective-access
  modules, reusing the existing `role_admin::helpers` API rather than copying
  request implementations.
- `persistence_web.rs`: privilege menu persistence and unsupported-key
  validation.
- `case_web.rs`: case-list effective access and workflow case-list access.
- `information_web.rs`: information and presave permissions.
- `transfer_web.rs`: import and export/submission read-versus-execute behavior.
- `terminology_web.rs`: terminology read, import, and approval behavior.
- `administration_web.rs`: users, roles, settings, and audit access.
- `dashboard_web.rs`: notice capabilities and e-mail permission persistence.

## Compatibility

All existing test function names and assertions are preserved unless a name
must be clarified after extraction. Production code, routes, fixtures,
permission profiles, and public interfaces remain unchanged.

Tests continue to use `#[serial]` where the original test did. No test is
silently dropped: a structural inventory compares test names before and after
the split.

## Assertion Improvements

During extraction, effective-access tests that currently assert only an
in-memory permission add Router assertions where a suitable existing endpoint
and fixture are already available. Denied Router responses assert HTTP 403;
granted requests assert the endpoint-specific success status or explicitly
assert that the response crossed the permission gate.

At least one representative denied response validates the JSON
permission-denied detail. Each area retains or gains an unrelated-permission
negative assertion to guard against privilege expansion.

No new database fixture framework or generalized route matrix is introduced.

## Verification

Before moving code, record all test function names in
`privilege_matrix_web.rs`. After extraction, verify the same names occur under
`effective_access/`, plus any explicitly added assertion tests.

Run the role-admin API tests, the complete `api` integration target, the
existing `authz` integration target, and the full `web-server` test suite.
Only pre-existing ignored or environment-dependent tests may remain skipped.
