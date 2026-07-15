# All-Section Portable Save Constraints Design

**Date:** 2026-07-15

## Goal

Apply Catalog-backed storage-representation validation to every editable Case
Editor section without moving business validation into the frontend.

The frontend must immediately show an error below an edited invalid field and
disable Save and Save Next. The backend must reject the same invalid value
before mutation when a client bypasses the frontend.

## Scope

The portable save gate includes only constraints that can be evaluated without
case-level business context or external terminology data:

- maximum length;
- primitive value type;
- portable formats, currently E2B datetime and base64;
- complete inline allowed-value sets;
- allowed nullFlavor values.

The gate excludes:

- required, mandatory, and conditional mandatory rules;
- cross-field and other business conditions;
- terminology and vocabulary lookup;
- submission and XML structural rules;
- future-date and other semantic validation.

Catalog authority metadata does not select frontend behavior. An ICH, FDA, or
MFDS representation constraint is attached to its concrete Case Editor field
and runs whenever that field is edited. No runtime authority branch is added.

## Sources Of Truth

Two distinct sources of truth are necessary:

1. The validation Catalog defines each rule's constraint and message.
2. A backend `PortableFieldBinding` manifest defines which concrete Case Editor
   field is governed by each portable Catalog rule.

The binding manifest does not duplicate rule values, limits, formats, or
messages. It contains only connection metadata:

```rust
PortableFieldBinding {
    section: "DG",
    frontend_path: "drugs[].dosageInformation[].doseValue",
    request_path: "dosageInformation[].doseValue",
    value_type: PortableValueType::Number,
    rule_codes: &["ICH.G.k.4.r.1a.ALLOWED.VALUE"],
    null_flavor_path: None,
}
```

All paths are explicit. The implementation must not infer aliases, replace a
concrete indexed path with an owner path, or use fallback path resolution.

## Generated Frontend Artifacts

The existing build-time exporter is extended to generate both:

- portable Catalog constraints for ICH, FDA, and MFDS;
- frontend field bindings grouped by Case Editor section.

The generated TypeScript remains deterministic and checked into the frontend
repository. `sync:validation-catalog` refreshes it and
`check:validation-catalog` detects drift. No runtime Catalog API, cache, ETag,
or manual copy step is introduced.

## Frontend Behavior

The existing `SyntaxIssue`, React Hook Form, and Case Editor validation state
are reused. A second validation store or new state framework is not added.

The generated bindings feed the existing generic Zod evaluator. It traverses
object, repeated, and nested repeated paths and emits concrete paths such as:

```text
drugs.1.dosageInformation.2.doseValue
```

Issue presentation and save state follow one rule:

- for a persisted case, only an issue whose concrete path is dirty is shown and
  blocks saving;
- for a new case without a persisted baseline, invalid supplied values are
  shown and block saving;
- absent optional values do not produce portable issues;
- required and semantic issues do not affect this save gate.

The existing `E2BFormField` error output moves below the field control. Compact
and table fields already using React Hook Form must expose their existing
`fieldState.error` at the corresponding control. The implementation adds only
targeted adapters where a bound field currently does not render that error.

When at least one visible portable issue exists, Save and Save Next are
disabled. Correcting all edited invalid fields clears their errors and enables
the buttons again. The existing save-time validation remains as a secondary
guard against programmatic invocation.

## Backend Behavior

The backend consumes the same binding manifest before database mutation.

- Direct page PATCH handlers validate only keys present in `changes`.
- Repeatable row create handlers validate supplied row values.
- Repeatable row PATCH handlers validate only changed values.
- Nested arrays preserve concrete indexes in reported paths.
- An absent optional value is ignored.
- A present value with the wrong primitive type is rejected.
- An explicit nullFlavor companion is resolved only through its declared path.

A shared adapter is called from the direct-page save path and the common
repeatable create/PATCH macros. This avoids per-section evaluator branches while
leaving model parsing and persistence ownership unchanged.

On failure, the backend uses the existing `400 Bad Request` response shape. The
message includes the Catalog rule code and concrete field path. No new
structured error protocol or frontend response parser is part of this work.
No mutation or validation-cache refresh occurs after a portable validation
failure.

## Coverage And Drift Prevention

Coverage tests classify every portable Catalog rule as exactly one of:

- bound to an editable Case Editor field; or
- explicitly excluded with a stable non-editable reason.

Silent omission is a test failure. A binding must reference an existing
portable Catalog rule, use a supported value type, and have a unique section and
path association. NullFlavor bindings must name their companion path
explicitly.

If authority-specific rules mapped to the same physical field have conflicting
inline values, primitive types, formats, or nullFlavor sets, generation fails.
The implementation must not silently apply their intersection. The conflict
must be resolved by binding the rule to its actual separate regional field or by
classifying it as authority-dependent business validation outside this gate.

Generated frontend tests verify that every backend binding is emitted once and
that no required, conditional, vocabulary, or semantic rule enters the portable
artifact.

## Testing

Backend tests cover:

- manifest-to-Catalog parity;
- direct-page changed-field validation;
- repeatable create and PATCH validation;
- nested repeated concrete paths;
- wrong primitive types;
- inline values, length, format, and nullFlavor;
- no-write behavior after rejection;
- unchanged legacy invalid values remaining editable.

Frontend tests cover:

- generated artifact drift;
- representative object, repeated, and nested repeated fields;
- dirty invalid fields receiving React Hook Form errors;
- errors rendering below their controls;
- Save and Save Next disabled while an edited field is invalid;
- existing untouched invalid values not displaying errors or blocking save;
- authority-independent portable evaluation;
- required and semantic rules not entering the portable save gate.

Each section must have at least one end-to-end binding vector. Each portable
constraint kind must have parity vectors that pass and fail identically in Rust
and TypeScript.

## Delivery Sequence

1. Generalize portable constraints from ICH-only to all Catalog authorities.
2. Add and validate the shared field binding manifest.
3. Extend deterministic TypeScript generation with section bindings.
4. Update the existing frontend evaluator and dirty-issue filtering.
5. Move existing field error rendering below controls and add targeted adapters.
6. Add the common backend direct and repeatable pre-mutation gate.
7. Migrate all Case Editor sections and enforce complete coverage tests.

The work is complete only when generated artifacts are in sync, all portable
Catalog rules are classified, every editable binding is covered, and backend
and frontend regression suites pass.
