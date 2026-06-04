# Presave One-Source Matrix Audit Design

## Problem

Presave alignment has allowed duplicate sources for the same reference field. The sender C.3.3.3 issue showed the failure mode: a hidden parent field and a visible child row both represented the same field, so validation and save behavior could disagree.

The audit rule is now stricter:

```text
one reference field
-> one canonical frontend source
-> one canonical backend source
-> one case import target, when imported
```

No dual source of truth is allowed. No hidden legacy write fields are allowed. Read aliases are allowed only when explicitly marked as migration-only compatibility and paired with a cleanup decision.

## Scope

Audit one presave type at a time. The recommended order is:

1. Sender
2. Study
3. Narrative
4. Receiver
5. Product
6. Reporter

Sender comes first because it already exposed duplicate parent/child storage, and `senderDepartment` still has an explicit parent-or-child matrix action that must be resolved.

## Matrix Additions

For each presave matrix row, add or verify these columns:

| Column | Purpose |
| --- | --- |
| `canonical frontend source` | The only form/type/schema field path allowed to author the value. |
| `canonical backend source` | The only DB/DTO/model path allowed to persist the presave value. |
| `allowed read aliases` | Compatibility-only input aliases, if any. Empty unless explicitly justified. |
| `allowed write keys` | Exact REST/write payload keys. Must not contain legacy duplicates. |
| `case import target` | Exact case section path if `referenceImportedToCase`; empty otherwise. |
| `duplicate sources found` | Parent/child, alias, fallback, or hidden-field conflicts discovered in code. |
| `decision` | Keep, migrate, remove, or preserve-only. |
| `tests required` | Mapper/form/API/import tests proving the decision. |

## Audit Rules

`referenceImportedToCase`:
- Exactly one canonical presave source.
- Exactly one case import target.
- Actual section import test must prove the import path.
- Write mapper must emit only the canonical backend key.

`referencePreserveOnly`:
- Exactly one canonical presave source.
- No case import path.
- Tests must prove preserve-only fields do not populate the case section.

`localSystemOnly`:
- Must be identity, metadata, row state, row order, linkage, audit, or UI mechanics.
- Must not look like a real reference/case content field.

`removed`:
- Must not exist in form controls, schema fields, canonical write mappers, REST DTOs, backend models, bootstrap schema, compatibility SQL, or section import code.
- Existing data migration/drop behavior must be explicit when a backend column or table is removed.

## Detection Pass

For each presave type:

1. Read the matrix and list every row by category.
2. Extract frontend type/schema/form fields.
3. Extract canonical read mapper aliases and write mapper keys.
4. Extract backend parent tables, child tables, DTO structs, BMC structs, and bootstrap compatibility SQL.
5. Extract case import code and actual section tests.
6. Flag any field with more than one source at the same layer, or more than one path across parent/child storage.
7. Resolve each flagged field in the matrix before implementation.

## Output Per Presave

Each presave audit produces a small report:

```text
Presave: <sender|study|...>

Confirmed 1-1 mappings:
- <field>: <frontend> -> <backend> -> <case target>

Duplicate sources found:
- <field>: <source A> and <source B>
  decision: <remove/migrate/keep one>

Removed fields still present:
- <field>: <file/path>

Tests to add or update:
- <test name and behavior>
```

## Acceptance Criteria

For one presave type to be considered aligned:

- Every matrix row has exactly one category.
- Every retained reference field has one canonical frontend source.
- Every retained reference field has one canonical backend source.
- Every imported field has one case import target and an actual section import test.
- Every removed field is absent from write paths, backend persistence, and UI/schema surfaces.
- Any read compatibility alias is explicitly listed as migration-only and does not write back.
- `rg` evidence confirms removed fields are gone except for migration/drop statements and historical tests explicitly named as legacy rejection tests.
