# RBAC Dynamic Role Tests Design

## Goal

Add reliable regression coverage for dynamic-role registration, update,
removal, full replacement, and precedence over built-in role permissions.

## Isolation

The dynamic-role registry is process-global. The scenarios therefore run
sequentially inside one integration test in
`crates/libs/lib-core/tests/rbac_dynamic_roles.rs`. Cargo runs that integration
test as a separate test binary, preventing its registry mutations from racing
with ACS unit tests in the library test binary.

The test clears the registry before the first assertion and uses a cleanup
guard that clears it again on normal return or panic. It does not depend on
execution order or state left by another test.

## Scenarios

The single test verifies these public API contracts in order:

1. `upsert_dynamic_role_permissions` registers a normalized custom role and
   `has_permission`, `has_any_permission`, and `has_all_permissions` observe it.
2. Upserting the same role replaces its prior permission vector.
3. `remove_dynamic_role` removes the custom role and restores the no-permission
   result for an unknown role.
4. `replace_dynamic_roles` atomically replaces the complete map: included roles
   become available and roles omitted from the replacement disappear.
5. A dynamic entry for `sponsor_admin_cro` takes precedence over its built-in
   administrator permissions. Removing that entry restores the built-in policy.

## Compatibility

Production RBAC code and public interfaces remain unchanged. The test uses
only exported ACS functions and permission constants. Existing policy snapshot
and ACS tests must remain unchanged and pass.

## Verification

Run the new integration test directly, then all ACS tests, then the complete
`lib-core` suite. The direct test must pass repeatedly without requiring
`--test-threads=1`.
