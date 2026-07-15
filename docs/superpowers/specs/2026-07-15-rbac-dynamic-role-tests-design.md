# RBAC Dynamic Role Tests Design

## Goal

Provide readable, failure-localized regression coverage for the dynamic-role
lifecycle and every menu-based permission-profile family. Tests are organized
by responsibility instead of combining the entire contract in one function or
one matrix test.

## File Structure

`crates/libs/lib-core/tests/rbac_dynamic_roles.rs` is only the integration-test
entry point. It declares modules stored under
`crates/libs/lib-core/tests/rbac_dynamic_roles/`:

- `support.rs`: registry reset guard and shared profile helpers.
- `registration.rs`: registration and role-name normalization.
- `update.rs`: replacement of permissions for an existing dynamic role.
- `removal.rs`: custom-role removal and resulting denial.
- `replacement.rs`: complete registry replacement and omitted-role removal.
- `precedence.rs`: dynamic definitions override built-in roles and removal
  restores the built-in policy.
- `case_profile.rs`: case read, edit, review, and lock profiles, including all
  nested case resources supplied by viewer and profile-edit bundles.
- `information_profile.rs`: presave template, sender, receiver, study, and
  narrative permission profiles.
- `transfer_profile.rs`: import and export/submission profiles and aliases.
- `administration_profile.rs`: users, audit, terminology, settings, roles, and
  administrator profiles.
- `dashboard_profile.rs`: workflow, notice, and e-mail profiles.

## Test Isolation

The dynamic-role registry is process-global. Every test acquires the existing
`serial_test` process lock, clears the registry before assertions, and owns a
cleanup guard whose `Drop` implementation clears the registry on normal return
or panic. Cargo runs this integration target in a process separate from ACS
unit tests, so mutations cannot race with library-test assertions.

Tests do not require the command-line `--test-threads=1` option.

## Lifecycle Contracts

Lifecycle files use exported ACS functions only. Together they verify:

- registration observes trimmed, case-insensitive role names;
- an upsert fully replaces the prior permission vector;
- removal denies an unknown custom role afterward;
- full replacement adds included roles and removes omitted roles;
- a dynamic `sponsor_admin_cro` definition fully replaces, rather than adds to,
  built-in permissions;
- removing that override restores built-in permissions.

## Permission Profile Contracts

Each profile test creates `AdminMenuPrivilege` values through
`permissions_for_menu_privileges`, installs the resulting vector as a dynamic
role, and checks access through `has_permission`, `has_any_permission`, and
`has_all_permissions` as appropriate.

Every profile file contains separate, named tests for its menu and checkbox
semantics. Positive assertions cover all permissions emitted by that profile;
negative assertions cover representative permissions belonging to disabled
checkboxes and unrelated profiles. Alias tests compare installed access for
each supported alias. Empty and unsupported menu profiles grant nothing.

The tests intentionally validate the complete public flow:

```text
menu privilege -> permission expansion -> dynamic role registration -> access check
```

## Compatibility

Production RBAC code and public interfaces remain unchanged. Existing policy
snapshot and ACS tests remain unchanged. The earlier single combined
`dynamic_role_lifecycle_and_builtin_precedence` test is deleted after its
coverage is distributed to the focused files.

## Verification

Run the dynamic-role integration target normally and twice consecutively, then
run all ACS tests and the complete `lib-core` suite. All new focused tests must
pass without state leakage; only existing database-dependent tests may remain
ignored.
