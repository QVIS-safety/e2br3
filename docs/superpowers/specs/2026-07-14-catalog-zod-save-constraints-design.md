# Catalog-Driven Zod And Save Gate

## Purpose

Use the backend Catalog as the source of truth for representation constraints
that must prevent an invalid Case Editor save. The frontend runs the portable
constraints through the existing Zod syntax flow for immediate feedback. The
backend evaluates the same Catalog constraints before a database write so a
direct API caller cannot bypass them.

This design intentionally does not add a runtime Catalog API, section loading,
caching, ETags, or terminology lookup.

## Scope

The save gate includes only:

- `max_length`
- primitive type and numeric shape
- portable `format`
- `inline_allowed_values`
- `null_flavor`

`inline_allowed_values` means the Catalog contains the complete closed set of
accepted values, such as `1,2,3,4` or `800,801,802,803,804,805`. It is not based
on an arbitrary list-size threshold.

The save gate excludes:

- vocabulary or terminology references, including MedDRA, EDQM, MFDS products,
  and WHODrug
- required and conditional-mandatory rules
- companion rules
- future-date and other business rules
- exporter and submission-readiness rules

Excluded rules continue through their existing validation and autocomplete
flows. They are not part of this change.

## Minimal Architecture

```text
Backend Catalog
   |                         |
   | generate                | evaluate directly
   v                         v
catalogConstraints.ts   validate_save_constraints(...)
   |                         |
   v                         v
existing Zod syntax      reject before DB write
```

There are two evaluators because the browser is not an integrity boundary, but
there is only one rule inventory: the backend Catalog.

## Generated Frontend Constraints

A small backend export command writes one generated TypeScript file:

`lib/zod/generated/catalogConstraints.ts`

The file contains only portable constraint data:

```ts
export const catalogConstraints = {
  "ICH.G.k.1.ALLOWED_VALUES": {
    kind: "inline_allowed_values",
    values: ["1", "2", "3", "4"],
    message: "G.k.1 must be one of: 1, 2, 3, 4.",
  },
  "ICH.G.k.2.1.1a.LENGTH.MAX": {
    kind: "max_length",
    maxLength: 10,
    message: "G.k.2.1.1a must be 10 characters or fewer.",
  },
} as const;
```

The generated file is committed but never edited manually. The exporter sorts
by rule code and supports a check mode that fails when regeneration changes the
file. No generated JSON, source-commit manifest, runtime API, or cross-repository
cache is introduced.

## Frontend Field Binding

The Catalog does not currently contain React Hook Form field paths. Existing
`lib/zod/sections/*.ts` definitions remain the frontend adapter and bind each
field to its Catalog rule code:

```ts
{
  field: "mpidVersion",
  ruleCode: "ICH.G.k.2.1.1a.LENGTH.MAX",
}
```

These section files own only UI field placement. They must not repeat
`maxLength`, allowed values, format patterns, nullFlavor policy, or messages.

The existing `lib/validation/syntax.ts` resolves `ruleCode` from the generated
map and constructs the corresponding Zod schema. Repeated and nested paths keep
using the existing section ruleset structure and concrete array indexes.

Required rules are removed from this save-blocking syntax path. Existing syntax
rules are removed only after their field has a Catalog rule binding and parity
test.

## Backend Save Gate

Add one reusable Rust evaluator:

```rust
validate_save_constraints(authorities, affected_values) -> Vec<ValidationIssue>
```

It reads the canonical Catalog directly and evaluates the same five constraint
kinds. It runs before the relevant Case mutation writes to the database. An
invalid request returns the existing REST validation error response and performs
no write.

Server-owned case or receiver policy determines authority. Request-provided
authority cannot remove regional constraints.

The first implementation must locate the narrowest existing shared mutation
boundary. If no single boundary exists, each affected mutation handler calls
the same evaluator and an inventory test proves coverage. This change does not
introduce a new mutation API or refactor unrelated routes.

## Parity

The exporter includes deterministic fixtures for every portable constraint
kind. Rust tests and TypeScript tests run the same valid and invalid values and
must agree on:

- pass or fail
- rule code
- concrete field path

Vocabulary fixtures are not included because vocabulary is outside this scope.

## Implementation Order

1. Identify the shared backend mutation boundary and current error response.
2. Add failing Rust tests proving invalid direct API input cannot be persisted.
3. Implement the five-kind Rust save evaluator and connect the mutation gate.
4. Add the generated TypeScript constraint map and regeneration check.
5. Add `ruleCode` binding to existing section Zod rules.
6. Update `syntax.ts` to evaluate generated constraints.
7. Add Rust/TypeScript parity fixtures and remove replaced handwritten values.

## Success Criteria

- Frontend save is blocked by Zod for all bound portable constraints.
- Direct API writes with the same invalid values are rejected before persistence.
- Constraint values and messages are not handwritten in frontend section files.
- `inline_allowed_values` includes only complete Catalog-owned closed sets.
- No vocabulary or terminology DB lookup is added.
- Required and business-validation issues remain draft-saveable.

## Implementation Status

The first ICH CI (`C.1`) vertical slice is implemented:

- the backend Catalog generates the frontend TypeScript constraint map;
- CI Zod bindings use generated max-length, format, inline allowed-value, and
  nullFlavor constraints;
- required CI values no longer enter the frontend syntax save gate;
- the CI direct PATCH path rejects bound invalid representations before its BMC
  write;
- vocabulary and terminology constraints remain excluded.

The generated frontend inventory contains portable ICH constraints for later
migration, but save-gate bindings are not yet complete for `RP`, `SD`, `LR`,
`SI`, `DM`, `DH`, `AE`, `LB`, `DG`, or `NR`. Those pages must not be described
as protected by this implementation.
