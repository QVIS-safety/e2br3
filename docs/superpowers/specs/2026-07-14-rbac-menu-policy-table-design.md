# RBAC Menu Policy Table Design

## Goal

Replace the repetitive control flow in `menu_policy.rs` with a declarative,
shared menu-policy structure while preserving every existing permission result,
permission order, public API, and unknown-key behavior.

## Scope

- Convert menu aliases and checkbox-to-permission mappings into static policy
  declarations.
- Support both fixed permission slices and existing computed role bundles such
  as viewer, profile-edit, and administrator permissions.
- Keep `AdminMenuPrivilege` and `permissions_for_menu_privileges` unchanged for
  callers.
- Keep deduplication stable: the first occurrence of a permission determines
  its output position.
- Preserve trimmed menu-key lookup and empty output for unknown or intentionally
  unsupported organization keys.

Route authorization, role semantics, permission constants, and capability
response shapes are outside this change.

## Design

Each menu family is represented by one `MenuPolicy` entry. An entry owns its
aliases and four checkbox mappings: read, edit, review, and lock. Each mapping
contains zero or more `PermissionSource` values.

`PermissionSource` has two forms:

- `Fixed(&'static [Permission])` for explicit permission constants.
- `Bundle(PermissionBundle)` for the existing viewer, profile-edit, or admin
  permission sets.

The evaluator finds the first policy whose aliases contain the trimmed menu
key, visits enabled checkbox mappings in the existing read/edit/review/lock
order, resolves each source, and appends permissions using the current stable
deduplication function.

This representation covers special behavior without a second procedural
fallback. For example, `case` maps read to the viewer bundle, edit to the
profile-edit bundle, and both review and lock to the same fixed approval slice.
`settings` and `roles` remain separate entries because they intentionally have
different semantics despite their former shared match arm.

## Compatibility Rules

- The exhaustive built-in-role and menu-policy fingerprint must remain exactly
  `5602083785880063594`.
- Existing targeted menu tests must continue to pass unchanged.
- Alias groups must expand to identical ordered permission vectors.
- No new public type is required; policy-table implementation types remain
  private to `menu_policy.rs`.

## Testing

Before implementation, add a structural test that requires a declarative
`MENU_POLICIES` table and rejects the legacy `match menu_key` dispatcher. Watch
that test fail against the current code.

After implementation, run the structural test, the exhaustive policy snapshot,
all ACS tests, and the complete `lib-core` test suite. A snapshot change is a
regression for this refactor and must not be accepted.

## Expected Result

The production portion of `menu_policy.rs` becomes data plus one generic
evaluator. Adding an alias or menu mapping becomes a localized declaration,
and omission audits can inspect a uniform structure instead of branching code.
