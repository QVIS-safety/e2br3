# RBAC Architecture Design

## Goal

Replace the 50 KB hand-maintained RBAC module with focused modules and declarative permission groups, then audit and correct proven policy gaps without mixing policy changes into the structural refactor.

## Current Problem

`model/acs/permission.rs` mixes the permission type system, 190 public constants, built-in role policies, menu privilege expansion, dynamic custom-role state, permission checks, and tests. CRUD constants and role sets repeat the same resource/action relationships in several places. Adding one resource therefore requires coordinated manual edits across unrelated concerns and makes omissions difficult to distinguish from intentional policy.

## Module Boundaries

```text
model/acs/
├── mod.rs
├── types.rs
├── catalog.rs
├── builtin_roles.rs
├── menu_policy.rs
├── dynamic_roles.rs
├── check.rs
└── tests.rs
```

- `types.rs`: `Resource`, `Action`, `Permission`, accessors, and display.
- `catalog.rs`: public permission constants generated from compact resource/action declarations. Existing constant names remain source-compatible.
- `builtin_roles.rs`: immutable system-admin, sponsor-admin, operational-edit, and viewer permission sets.
- `menu_policy.rs`: `AdminMenuPrivilege` and menu checkbox expansion only.
- `dynamic_roles.rs`: normalized custom-role cache mutation and lookup.
- `check.rs`: `role_permissions`, `has_permission`, `has_any_permission`, and `has_all_permissions`.
- `tests.rs`: built-in role, menu matrix, dynamic override, and policy-audit contracts.
- `mod.rs`: private modules plus the same public re-exports currently provided by `acs::*`.

## Phase 1: Behavior-Preserving Refactor

Before moving code, capture exhaustive ordered permission snapshots for every built-in role, menu key, and checkbox combination. Generate CRUD constants through a local macro while preserving every public constant name and `(Resource, Action)` value. Split responsibilities without changing role membership, dynamic-role precedence, menu aliases, permission ordering, or unknown-role behavior.

The dynamic-role implementation may gain a shared internal lookup helper, but its externally observable precedence remains: an exact normalized dynamic role overrides the built-in role; otherwise built-in policy applies.

## Phase 2: Policy Audit and Corrections

Audit questionable differences only after Phase 1 is green. Each correction requires a failing policy test, one policy change, and an independent commit. Initial audit targets are:

- viewer access to drug device characteristics;
- sponsor-admin access to the reserved e-mail send permission;
- system-admin reliance on separate endpoint guards rather than operational permissions;
- the DELETE asymmetry between operational edit groups and presave templates;
- equivalent menu aliases producing equivalent permission sets;
- poisoned dynamic-role lock behavior.

Absence is not automatically a bug. A candidate changes only when code usage and product policy establish the expected permission. Unproven candidates remain unchanged and are reported.

## Compatibility and Verification

- Existing imports through `lib_core::model::acs::*` continue to compile.
- Phase 1 snapshots must be identical before and after the split.
- Run lib-core RBAC tests, permission-profile tests, web-server role-admin tests, and authorization tests.
- Measure source characters across the entire `model/acs/` directory, not only the removed monolithic file.
- No database schema, API payload, role identifier, or menu key changes are part of the structural phase.
